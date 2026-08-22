//! Minimal C ABI over versioned UTF-8 JSON contracts.

use std::ffi::{CStr, CString, c_char};

use serde::Serialize;
use serde_json::Value;
use typikon_core::Engine;
use typikon_loader::{MemoryResource, SchemaKind, load_pack, validate_value};
use typikon_schema::{FFI_RESPONSE_SCHEMA, ResourceBundle};

#[derive(Debug, Serialize)]
struct FfiResponse {
    schema: &'static str,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<FfiError>,
}

#[derive(Debug, Serialize)]
struct FfiError {
    code: &'static str,
    message: String,
}

impl FfiResponse {
    fn success(plan: Value) -> Self {
        Self {
            schema: FFI_RESPONSE_SCHEMA,
            ok: true,
            plan: Some(plan),
            error: None,
        }
    }

    fn failure(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            schema: FFI_RESPONSE_SCHEMA,
            ok: false,
            plan: None,
            error: Some(FfiError {
                code,
                message: message.into(),
            }),
        }
    }
}

/// Compiles a service through the generic engine using serialized resources.
///
/// The returned string is allocated by this library and must be passed exactly
/// once to [`typikon_string_free`]. All expected failures are returned as a
/// `typikon.ffi-response/v0.1` JSON error envelope. Panics are caught before
/// crossing the ABI boundary.
///
/// # Safety
///
/// Each input must be a valid pointer to a NUL-terminated byte string and must
/// remain readable for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typikon_compile(
    resource_bundle_json: *const c_char,
    request_json: *const c_char,
) -> *mut c_char {
    let response = std::panic::catch_unwind(|| {
        // SAFETY: The caller obligations are documented by this public unsafe
        // function and are applied only while reading each input.
        unsafe { compile_from_pointers(resource_bundle_json, request_json) }
    })
    .unwrap_or_else(|_| FfiResponse::failure("panic", "internal panic was contained"));
    response_pointer(&response)
}

/// Releases a string returned by [`typikon_compile`].
///
/// # Safety
///
/// `value` must be null or a pointer returned by [`typikon_compile`] that has
/// not previously been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typikon_string_free(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: The caller guarantees this is an owned pointer returned by
        // CString::into_raw in response_pointer and that it is freed once.
        drop(unsafe { CString::from_raw(value) });
    }
}

unsafe fn compile_from_pointers(
    resource_bundle_json: *const c_char,
    request_json: *const c_char,
) -> FfiResponse {
    // SAFETY: This helper is called only under typikon_compile's documented
    // pointer contract.
    let bundle_text = match unsafe { utf8_input(resource_bundle_json, "resource_bundle_json") } {
        Ok(value) => value,
        Err(response) => return response,
    };
    // SAFETY: Same contract as above, applied to the second input.
    let request_text = match unsafe { utf8_input(request_json, "request_json") } {
        Ok(value) => value,
        Err(response) => return response,
    };
    compile_text(bundle_text, request_text)
}

unsafe fn utf8_input<'a>(value: *const c_char, name: &'static str) -> Result<&'a str, FfiResponse> {
    if value.is_null() {
        return Err(FfiResponse::failure(
            "null_input",
            format!("{name} must not be null"),
        ));
    }
    // SAFETY: The caller guarantees a readable NUL-terminated byte string.
    unsafe { CStr::from_ptr(value) }.to_str().map_err(|error| {
        FfiResponse::failure("invalid_utf8", format!("{name} is not UTF-8: {error}"))
    })
}

fn compile_text(bundle_text: &str, request_text: &str) -> FfiResponse {
    let bundle_value: Value = match serde_json::from_str(bundle_text) {
        Ok(value) => value,
        Err(error) => return FfiResponse::failure("malformed_bundle", error.to_string()),
    };
    if let Err(error) = validate_value(
        SchemaKind::ResourceBundle,
        "native resource bundle",
        &bundle_value,
    ) {
        return FfiResponse::failure("invalid_bundle", error.to_string());
    }
    let bundle: ResourceBundle = match serde_json::from_value(bundle_value) {
        Ok(value) => value,
        Err(error) => return FfiResponse::failure("invalid_bundle", error.to_string()),
    };
    let resource = MemoryResource::from_text(bundle.files);
    let pack = match load_pack(&resource) {
        Ok(value) => value,
        Err(error) => return FfiResponse::failure("invalid_pack", error.to_string()),
    };
    let plan_json = match Engine::new(pack).compile_service_json(request_text) {
        Ok(value) => value,
        Err(error) => return FfiResponse::failure("compile_failed", error.to_string()),
    };
    match serde_json::from_str(&plan_json) {
        Ok(value) => FfiResponse::success(value),
        Err(error) => FfiResponse::failure("invalid_plan_json", error.to_string()),
    }
}

fn response_pointer(response: &FfiResponse) -> *mut c_char {
    let json = serde_json::to_string(response).expect("FFI response always serializes");
    CString::new(json)
        .expect("serialized JSON contains no interior NUL")
        .into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use typikon_schema::{REQUEST_SCHEMA, RESOURCE_BUNDLE_SCHEMA};

    fn bundle_json() -> CString {
        let bundle = serde_json::json!({
            "schema": RESOURCE_BUNDLE_SCHEMA,
            "files": {
                "pack.yaml": include_str!("../tests/fixtures/minimal-pack/pack.yaml"),
                "services/vespers.yaml": include_str!("../tests/fixtures/minimal-pack/services/vespers.yaml"),
                "observances/saint.yaml": include_str!("../tests/fixtures/minimal-pack/observances/saint.yaml"),
                "rules/ordinary.yaml": include_str!("../tests/fixtures/minimal-pack/rules/ordinary.yaml")
            }
        });
        CString::new(bundle.to_string()).unwrap()
    }

    fn request_json() -> CString {
        CString::new(
            serde_json::json!({
                "schema": REQUEST_SCHEMA,
                "civil_date": "2026-07-25",
                "service": "great_vespers"
            })
            .to_string(),
        )
        .unwrap()
    }

    unsafe fn take_response(pointer: *mut c_char) -> Value {
        assert!(!pointer.is_null());
        // SAFETY: The pointer is the live return value from typikon_compile.
        let value =
            serde_json::from_str(unsafe { CStr::from_ptr(pointer) }.to_str().unwrap()).unwrap();
        // SAFETY: This is the only release of the returned pointer.
        unsafe { typikon_string_free(pointer) };
        value
    }

    #[test]
    fn native_boundary_compiles_and_returns_an_owned_schema_valid_response() {
        let bundle = bundle_json();
        let request = request_json();
        // SAFETY: Both CString inputs satisfy the ABI pointer contract.
        let pointer = unsafe { typikon_compile(bundle.as_ptr(), request.as_ptr()) };
        // SAFETY: The pointer is consumed exactly once by the helper.
        let response = unsafe { take_response(pointer) };

        validate_value(SchemaKind::FfiResponse, "FFI success", &response).unwrap();
        validate_value(SchemaKind::Plan, "FFI plan", &response["plan"]).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["plan"]["pack"]["id"], "ffi-test");
        assert_eq!(response["plan"]["day"]["tone"], "tone_7");
    }

    #[test]
    fn native_boundary_returns_structured_errors_and_accepts_null_free() {
        let request = request_json();
        // SAFETY: A null bundle pointer is an explicitly supported error case;
        // the request CString satisfies the other input contract.
        let pointer = unsafe { typikon_compile(std::ptr::null(), request.as_ptr()) };
        // SAFETY: The pointer is consumed exactly once by the helper.
        let response = unsafe { take_response(pointer) };

        validate_value(SchemaKind::FfiResponse, "FFI error", &response).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "null_input");
        // SAFETY: The free function explicitly accepts null.
        unsafe { typikon_string_free(std::ptr::null_mut()) };
    }
}

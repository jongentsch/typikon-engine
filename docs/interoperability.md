# Interoperability

The Rust-native API uses rich types. Foreign-language interfaces stay
deliberately small and exchange UTF-8 JSON conforming to the request and plan
contracts. `Engine::compile_service_json` is the implemented shared boundary:
it validates `typikon.request/v0.1`, compiles through the same typed evaluator,
validates `typikon.plan/v0.1`, and emits deterministic compact JSON.

Minimal request:

```json
{
  "schema": "typikon.request/v0.1",
  "civil_date": "2026-07-25",
  "service": "great_vespers"
}
```

`tone` and `phase` may be supplied as assertions. `observances` may select
explicit context; when omitted it defaults to an empty list and the engine
performs fixed-date discovery. The compiled plan preserves that exact caller
intent. Automatically selected observances appear in the result with a
`selection_derivation`; they are not rewritten into the recorded request.

The `typikon-ffi` crate now builds `cdylib` and `staticlib` artifacts and exports
exactly two C functions (declared in `crates/typikon-ffi/include/typikon.h`):

```c
char *typikon_compile(const char *resource_bundle_json, const char *request_json);
void typikon_string_free(char *value);
```

The first input conforms to `typikon.resource-bundle/v0.1`: a map of safe
relative paths to UTF-8 YAML strings, including `pack.yaml`. The second conforms
to `typikon.request/v0.1`. The returned allocation always contains a
`typikon.ffi-response/v0.1` success or error envelope and must be released once
with `typikon_string_free`. Null and invalid UTF-8 inputs become structured
errors, and panics are caught before they can cross the ABI boundary.

No Rust layout, borrowed pointer, filesystem path, or internal object graph
crosses the boundary. `examples/ffi_smoke.py` is a non-Rust consumer using only
Python's standard `ctypes` and `json` modules; CI builds the shared library and
runs that consumer on Linux.

For WebAssembly, a wrapper can construct a `MemoryResource` from JavaScript-
supplied files, load it, compile, and serialize the plan. Filesystem support is
feature-gated in `typikon-loader`, so browser builds do not require directory
access. Language-specific wrappers such as PyO3 can remain independent.

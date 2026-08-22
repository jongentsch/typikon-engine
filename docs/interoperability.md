# Interoperability

The Rust-native API uses rich types. Future foreign-language interfaces should
stay deliberately small and exchange UTF-8 JSON conforming to the request and
plan contracts.

A future native boundary can expose operations conceptually equivalent to:

```text
validate_pack(pack_resource_json) -> result_json
compile_service(pack_resource_json, request_json) -> plan_json
free_string(pointer)
```

No Rust layout, borrowed pointer, or internal object graph should cross that
boundary. A `cdylib`/`staticlib` wrapper can depend on `typikon-core` without
changing it.

For WebAssembly, a wrapper can construct a `MemoryResource` from JavaScript-
supplied files, load it, compile, and serialize the plan. Filesystem support is
feature-gated in `typikon-loader`, so browser builds do not require directory
access. Language-specific wrappers such as PyO3 can remain independent.

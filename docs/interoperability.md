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

A native boundary can therefore expose operations equivalent to:

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

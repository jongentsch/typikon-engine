# Typikon Engine

[![CI](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml)

`typikon-engine` is a schema-first Orthodox liturgical compiler. It loads a
human-maintained jurisdiction pack at runtime and deterministically compiles a
civil date into service plans. Jurisdiction data is not compiled into Rust.

The repositories are peers:

- `typikon-engine`: schemas, validation, evaluator, CLI, and FFI;
- `typikon-goarch`: experimental GOARCH pack;
- `typikon-oca`: experimental OCA pack;
- `typikon-antiochian`: experimental Antiochian pack.

## Mental model

- A `typikon.service/v0.2` document is the ordered service book: every fixed
  and changeable component of Vespers, Matins, or Divine Liturgy is visible.
- A `typikon.observance/v0.4` document owns its date, rank, common material,
  and the actual material it contributes to each service component.
- A `typikon.rank/v0.1` profile states which changeable components that rank
  must supply for a service plan to be complete.
- A `typikon.rule/v0.3` document appoints cycle material, selects a service
  form, or admits an observance's own material into a component.
- Authority documents record sources, scoped claims, and dated witnesses.

There is no feast slot, `complete-propers` blob, or global leaf-resource ID to
copy between files. An Apolytikion may be written once under an observance's
`common` map and reused locally with `use: common.apolytikion`.

The output is `typikon.plan/v0.2`. It contains the entire ordered service, not
only emitted propers. Fixed components are `resolved`; optional empty
components are `omitted`; and required material that has not been normalized
is `unresolved`. Any unresolved requirement makes the plan
`requires_review`, so a dated webpage cannot silently stand in as perennial
data.

## Try it

```console
cargo run -p typikon-cli -- validate ../typikon-goarch

cargo run -p typikon-cli -- compile-date \
  --pack ../typikon-goarch \
  --date 2026-12-25
```

`compile_date` needs only a pack and target date. It discovers fixed and
Paschal-offset observances, adjusts preceding-evening services, and returns a
service-keyed map. The lower-level request boundary remains
`typikon.request/v0.1` for compiling one named service.

The current three packs establish date selection and complete service
structures for twelve major feasts. Their individual major-feast propers are
deliberately unresolved until component-level perennial data is transcribed
from authoritative sources. The ordinary Lord-I-Call witnesses demonstrate
direct observance-owned material.

## Verify

```console
cargo test --workspace
cargo test --workspace --features conformance
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
```

The conformance suite expects the three pack repositories beside the engine;
`TYPIKON_GOARCH_PACK`, `TYPIKON_OCA_PACK`, and `TYPIKON_ANTIOCHIAN_PACK` can
override their locations.

See [architecture](docs/architecture.md),
[schema philosophy](docs/schema-philosophy.md),
[liturgist workflow](docs/liturgist-workflow.md), and the
[complete-service roadmap](docs/complete-service-plan-roadmap.md).

## License

License selection is pending.

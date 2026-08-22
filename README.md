# Typikon Engine

[![CI](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml)

`typikon-engine` is the Rust-engine repository in a three-repository project:

- `typikon-engine/` (this directory): generic loader, evaluator, CLI, and schemas;
- `typikon-goarch/`: external experimental GOARCH runtime resource pack;
- `typikon-oca/`: external experimental OCA runtime resource pack.

It is a schema-first Rust spike for a reusable Orthodox Typikon
Engine / liturgical compiler. It loads versioned, human-maintained tradition
definitions at runtime and produces deterministic semantic plans. It does not
ship liturgical texts or encode a jurisdiction's rules in Rust.

The present milestone deliberately covers only one service fragment:
ordinary Saturday-evening Great Vespers, `Lord, I Call`, with its stichera,
`Glory`, and `Both now` slots. The example GOARCH and OCA packs are small
research fixtures, not usable or complete typika.

Evidence is categorized as a retrievable `source`, a reusable `scoped_claim`,
or a date-specific `dated_witness`; authoritative standing versus observed
behavior is recorded separately.

## Try the spike

```console
cargo run -p typikon-cli -- validate ../typikon-goarch

cargo run -p typikon-cli -- compile-service \
  --pack ../typikon-goarch \
  --date 2026-07-25 \
  --service great_vespers
```

The OCA pack contains two dated fixtures grounded in official service orders.
The calendar selects the matching observance automatically:

```console
cargo run -p typikon-cli -- compile-service \
  --pack ../typikon-oca \
  --date 2023-08-26 \
  --service great_vespers
```

The engine calculates Orthodox Pascha, the Triodion/Pentecostarion/ordinary
phase, and the ordinary Octoechos tone from the date. `--tone` and `--phase`
are optional assertions against those results. Pascha through Bright Saturday
has an explicit `null` ordinary tone rather than a fabricated mode.

Run the complete verification suite with:

```console
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

These default tests are self-contained and pass in a standalone engine clone.
Cross-repository conformance tests are deliberately explicit. With the GOARCH
and OCA repositories checked out beside this repository, run:

```console
cargo test -p typikon-core \
  --features conformance \
  --test external_packs
```

`TYPIKON_GOARCH_PACK` and `TYPIKON_OCA_PACK` may be set to alternate pack
directories. CI checks out all three repositories side by side and runs this
target separately from the standalone suite.

## Workspace boundary

- `typikon-schema`: language-neutral contract represented as Rust data types.
- `typikon-loader`: resource abstraction, YAML parsing, JSON Schema validation,
  and reference validation.
- `typikon-core`: pure, deterministic matching and plan assembly.
- `typikon-cli`: filesystem-backed development harness.

`typikon-core::Engine::compile_service_json` is the versioned, deterministic
UTF-8 JSON boundary intended for non-Rust wrappers. Requests identify
`typikon.request/v0.1`; results identify `typikon.plan/v0.1` and are validated
before being returned.

The core receives a validated in-memory definition model. It neither reads the
filesystem nor fetches the network. See [architecture](docs/architecture.md),
[calendar model](docs/calendar-model.md),
[interoperability](docs/interoperability.md),
[schema philosophy](docs/schema-philosophy.md), and
[fixture evidence](docs/fixture-evidence.md).

## License

License selection is pending. Apache-2.0, MPL-2.0, and GPL-3.0 remain under
consideration; no license has been selected by this spike.

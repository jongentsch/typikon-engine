# Typikon Engine

[![CI](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/jongentsch/typikon-engine/actions/workflows/ci.yml)

`typikon-engine` is the Rust-engine repository in a four-repository project:

- `typikon-engine/` (this directory): generic loader, evaluator, CLI, and schemas;
- `typikon-goarch/`: external experimental GOARCH runtime resource pack;
- `typikon-oca/`: external experimental OCA runtime resource pack;
- `typikon-antiochian/`: external experimental Antiochian runtime resource pack.

It is a schema-first Rust spike for a reusable Orthodox Typikon
Engine / liturgical compiler. It loads versioned, human-maintained tradition
definitions at runtime and produces deterministic semantic plans. It does not
ship liturgical texts or encode a jurisdiction's rules in Rust.

The detailed normalized milestone covers ordinary Saturday-evening Great
Vespers, `Lord, I Call`, with its stichera, `Glory`, and `Both now` slots. The
example GOARCH, OCA, and Antiochian packs also provide whole-service
proper-bundle baselines for Vespers, Matins, and Divine Liturgy for twelve major
feasts. Each observance appoints typed, evidence-bearing resources to the
three services, and rules place those appointments into service slots. Those
resources retain official external references rather than copying hymn text or
claiming that every service-book element has already been normalized. The packs
remain research fixtures, not usable or complete typika.

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

The major-feast conformance matrix compiles twelve feasts through three
services in each pack. Fixed feasts and the three Paschal-offset feasts are
automatically discovered. The date-level facade adjusts the preceding civil
evening for Vespers and returns a deterministic service-keyed map:

```console
cargo run -p typikon-cli -- compile-date \
  --pack ../typikon-oca \
  --date 2026-12-25
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
Cross-repository conformance tests are deliberately explicit. With the GOARCH,
OCA, and Antiochian repositories checked out beside this repository, run:

```console
cargo test -p typikon-core \
  --features conformance \
  --test external_packs
```

`TYPIKON_GOARCH_PACK`, `TYPIKON_OCA_PACK`, and `TYPIKON_ANTIOCHIAN_PACK` may
be set to alternate pack directories. CI checks out all four repositories side
by side and runs this target separately from the standalone suite.

## Workspace boundary

- `typikon-schema`: language-neutral contract represented as Rust data types.
- `typikon-loader`: resource abstraction, YAML parsing, JSON Schema validation,
  and reference validation.
- `typikon-core`: pure, deterministic matching and plan assembly.
- `typikon-cli`: filesystem-backed development harness.
- `typikon-ffi`: minimal `cdylib`/`staticlib` C ABI over UTF-8 JSON.

`typikon-core::Engine::compile_date` is the pack-plus-date facade for callers
that want every matching service without naming a feast. The lower-level
`compile_service_json` method remains the versioned, deterministic UTF-8 JSON
boundary intended for non-Rust wrappers. Requests identify
`typikon.request/v0.1`; results identify `typikon.plan/v0.1` and are validated
before being returned.

An emitted plan item can be static rule material or a resolved observance
appointment. Resolved resources are self-describing in the output: resource
ID, title, kind, role, official reference, observance, and combined authority
provenance are all retained.

Build and exercise the native ABI from Python with:

```console
cargo build --release -p typikon-ffi
python examples/ffi_smoke.py target/release/typikon_ffi.dll
```

Linux uses `target/release/libtypikon_ffi.so`; macOS uses
`target/release/libtypikon_ffi.dylib`.

The core receives a validated in-memory definition model. It neither reads the
filesystem nor fetches the network. See [architecture](docs/architecture.md),
[calendar model](docs/calendar-model.md),
[interoperability](docs/interoperability.md),
[schema philosophy](docs/schema-philosophy.md), and
[fixture evidence](docs/fixture-evidence.md). The staged path from the current
whole-service proper bundles to fully normalized readings and hymnody is in the
[complete service plan roadmap](docs/complete-service-plan-roadmap.md).

## License

License selection is pending. Apache-2.0, MPL-2.0, and GPL-3.0 remain under
consideration; no license has been selected by this spike.

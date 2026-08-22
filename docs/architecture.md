# Architecture

## Boundary

```text
caller-supplied TraditionResource
              |
              v
 YAML parse + JSON Schema validation
              |
              v
 typed, reference-checked LoadedPack
              |
              v
 deterministic typikon-core evaluation
              |
              v
 typed typikon.plan/v0.1 value
```

`TraditionResource` supplies relative resource names and bytes. The loader
includes a memory implementation and, behind its `filesystem` feature, a
directory implementation. Directory paths are confined to the supplied pack
root. Other providers (archive, browser fetch result, database, embedded bytes)
can implement the same interface without changing evaluation.

Tradition resources are never compiled into `typikon-core`. In this project,
`typikon-engine`, `typikon-goarch`, and `typikon-oca` are peer directories so
that repository ownership is also visible in the filesystem. The JSON Schemas
are engine contracts and may be embedded in loader artifacts; the GOARCH and
OCA data remain external runtime inputs.

## Minimal compilation flow

1. Resolve the service's `liturgical_day_offset` against the requested civil
   date.
2. Compute the liturgical weekday.
3. Use explicitly selected observances, or discover fixed-date observances.
4. Match structured `when` and `unless` predicates.
5. Resolve only documented variable references such as `$day.tone`; there is
   no expression language.
6. Validate every emitted section and slot against the service definition.
7. Reject collisions in `optional` and `one` slots as ambiguity.
8. Return items plus rule and authority provenance.

Rules and loaded records are held in ordered maps. Observances are sorted, and
decisions are numbered in deterministic evaluation order.

An observance predicate in `when` binds the matching observance for material
variables. An observance predicate in `unless` checks the whole selected
liturgical context, so the presence of another observance can exclude a rule.

## Deliberately deferred

- calendar conversion and complete fixed-calendar semantics;
- Orthodox Pascha and tone-cycle derivation (the spike accepts `tone` as
  request context);
- conjunction, precedence, transfer, vigil, and discretionary-choice models;
- native ABI, WASM, Python, and other wrappers.

These are deferred rather than represented by guessed abstractions.

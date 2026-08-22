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
2. Project the liturgical date into the pack's Gregorian, Revised Julian, or
   Julian fixed calendar.
3. Calculate Orthodox Pascha and the pack-mapped Octoechos tone.
4. Compute the liturgical weekday.
5. Use explicitly selected observances, or discover fixed-date observances in
   the projected calendar.
6. Match structured `when` and `unless` predicates.
7. Resolve only documented variable references such as `$day.tone`; there is
   no expression language.
8. Validate every emitted section and slot against the service definition.
9. Reject collisions in `optional` and `one` slots as ambiguity.
10. Return component derivations plus combined scoped-claim and dated-witness
   provenance. A
   scoped claim links back to the source records from which it was derived.

Rules and loaded records are held in ordered maps. Observances are sorted, and
decisions are numbered in deterministic evaluation order.

An observance predicate in `when` binds the matching observance for material
variables. An observance predicate in `unless` checks the whole selected
liturgical context, so the presence of another observance can exclude a rule.

The fixed calendar, Paschalion, and tone cycle are independent pack settings.
Old Calendar support therefore means `fixed: julian`; it is not a separate
engine branch and does not change the Julian-based Orthodox Paschalion. See the
[calendar model](calendar-model.md) for algorithms, evidence, and limits.

## Deliberately deferred

- conjunction, precedence, transfer, vigil, and discretionary-choice models;
- Triodion/Pentecostarion phase derivation and special-period tone semantics;
- native ABI, WASM, Python, and other wrappers.

These are deferred rather than represented by guessed abstractions.

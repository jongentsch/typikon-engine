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

Definition directories are scanned recursively. Packs can therefore keep
observances in a human-scale taxonomy such as `observances/feasts/major/`,
`observances/feasts/minor/`, and `observances/saints/<type>/`. An observance's
stable `id`, rather than its file path, remains the reference key, so moving a
definition between taxonomy directories does not change compilation behavior.

Tradition resources are never compiled into `typikon-core`. In this project,
`typikon-engine`, `typikon-goarch`, `typikon-oca`, and `typikon-antiochian` are
peer directories so that repository ownership is also visible in the
filesystem. The JSON Schemas
are engine contracts and may be embedded in loader artifacts; the GOARCH, OCA,
and Antiochian data remain external runtime inputs.

## Minimal compilation flow

1. Resolve the service's `liturgical_day_offset` against the requested civil
   date.
2. Project the liturgical date into the pack's Gregorian, Revised Julian, or
   Julian fixed calendar.
3. Calculate Orthodox Pascha, the Paschal-cycle phase, and the pack-mapped
   Octoechos tone when that ordinary cycle is active.
4. Compute the liturgical weekday.
5. Use explicitly selected observances, or discover fixed-date and
   `pascha_offset_days` observances from the calculated day.
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

The foreign-language boundary accepts a versioned request JSON document and
returns a schema-validated plan JSON document. Automatic context discovery is
recorded as output provenance and does not mutate the caller's recorded input.

The date-level facade accepts a target calendar date, derives each service's
civil start date from `liturgical_day_offset`, and compiles every matching
service with an empty observance list. It returns a deterministic map of
individually schema-valid plans keyed by service ID; it introduces no second
plan contract.

An observance predicate in `when` binds the matching observance for material
variables. An observance predicate in `unless` checks the whole selected
liturgical context, so the presence of another observance can exclude a rule.

The fixed calendar, Paschalion, and tone cycle are independent pack settings.
Old Calendar support therefore means `fixed: julian`; it is not a separate
engine branch and does not change the Julian-based Orthodox Paschalion. See the
[calendar model](calendar-model.md) for algorithms, evidence, and limits.

## Deliberately deferred

- conjunction, precedence, transfer, vigil, and discretionary-choice models;
- typed decomposition of whole-service major-feast proper bundles beyond the
  currently normalized `Lord, I Call` fragment;
- WebAssembly and mature language-specific wrapper packages.

These are deferred rather than represented by guessed abstractions.

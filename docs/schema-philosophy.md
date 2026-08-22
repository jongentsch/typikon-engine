# Schema philosophy

YAML is the authoring form. JSON Schema is the external definition contract,
and typed Rust structures are the evaluator input. Generic YAML maps are used
only at the validation boundary and for deliberately open-ended observance
properties and semantic material attributes.

The initial vocabulary is intentionally small: pack, service, observance, rule,
authority, compile request, and plan. Every serialized input and output names
its schema version. Rank and tone values remain pack strings rather than Rust
enums. Material describes what a consumer must supply; it contains no prayer or
hymn text.

Authority provenance is a small typed graph. A `source` has a retrievable
reference. A `scoped_claim` contains the reusable assertion and cites one or
more source authority IDs. A `dated_witness` has a retrievable reference and a
required liturgical date. Its separate `kind` says whether the record is
authoritative or merely observed behavior. The loader validates both the
category-specific shape and claim-to-source references.

Rules contain equality predicates and emissions. Values in emissions may be
literal scalars or exact variable references. Arbitrary expressions, scripts,
callbacks, and magic numeric priorities are unsupported.

Calendar configuration uses closed enums for algorithms but keeps the eight
tone names in pack vocabulary. Compiled plans expose ordered component
derivations rather than hiding calendar conversion, Pascha, weekday, tone, or
calculated phase behind a single opaque result. The ordinary tone is nullable
only for Pascha through Bright Saturday, when the cycle is explicitly
suspended.

The `observance.properties` object is an escape hatch for evidence-led pack
experimentation. A property should graduate into a first-class concept only
after multiple real fixtures establish shared semantics.

# Schema philosophy

YAML is the authoring form. JSON Schema is the external definition contract,
and typed Rust structures are the evaluator input. Generic YAML maps are used
only at the validation boundary and for deliberately open-ended observance
properties and semantic material attributes.

The initial vocabulary is intentionally small: pack, service, observance, rule,
authority, and plan. Rank and tone values remain pack strings rather than Rust
enums. Material describes what a consumer must supply; it contains no prayer or
hymn text.

Rules contain equality predicates and emissions. Values in emissions may be
literal scalars or exact variable references. Arbitrary expressions, scripts,
callbacks, and magic numeric priorities are unsupported.

The `observance.properties` object is an escape hatch for evidence-led pack
experimentation. A property should graduate into a first-class concept only
after multiple real fixtures establish shared semantics.

# Schema philosophy

YAML is the authoring form. JSON Schema is the external definition contract,
and typed Rust structures are the evaluator input. Generic YAML maps are used
only at the validation boundary and for deliberately open-ended observance
properties and semantic material attributes.

The vocabulary is intentionally small: pack, service, observance, liturgical
resource, rule, authority, compile request, and plan. Every serialized input
and output names its schema version. Rank and tone values remain pack strings
rather than Rust enums. Material describes what a consumer must supply; it
contains no prayer or hymn text.

Authority provenance is a small typed graph. A `source` has a retrievable
reference. A `scoped_claim` contains the reusable assertion and cites one or
more source authority IDs. A `dated_witness` has a retrievable reference and a
required liturgical date. Its separate `kind` says whether the record is
authoritative or merely observed behavior. The loader validates both the
category-specific shape and claim-to-source references.

Rules contain equality predicates and emissions. An emission contains exactly
one source: either static `material`, whose values may be literal scalars or
exact variable references, or an `appointment` role resolved through the bound
observance and current service. Arbitrary expressions, scripts, callbacks, and
magic numeric priorities are unsupported.

Appointments keep ownership at the right layer. A service defines available
sections and slots. An observance owns its date, rank, and service-specific
resource appointments. A rule decides which appointment role fills which slot.
A `typikon.resource/v0.1` document owns the material's stable ID, title, kind,
role, official external reference, and authority provenance. The loader rejects
unknown resource IDs and role mismatches before compilation.

Calendar configuration uses closed enums for algorithms but keeps the eight
tone names in pack vocabulary. Compiled plans expose ordered component
derivations rather than hiding calendar conversion, Pascha, weekday, tone, or
calculated phase behind a single opaque result. The ordinary tone is nullable
only for Pascha through Bright Saturday, when the cycle is explicitly
suspended.

The `observance.properties` object is an escape hatch for evidence-led pack
experimentation. A property should graduate into a first-class concept only
after multiple real fixtures establish shared semantics.

The three-pack major-feast conformance matrix established Paschal offsets as a
shared concept, so every observance date is an
explicit choice between `date.fixed` and `date.paschal_offset`. The latter is
an integer count of elapsed days from calculated Orthodox Pascha: Palm Sunday
is `-7`, Ascension is `39`, and Pentecost is `49`. Selection behavior no longer
depends on an engine-interpreted property.

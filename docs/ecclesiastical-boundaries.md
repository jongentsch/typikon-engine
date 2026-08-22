# Ecclesiastical boundaries

The engine evaluates one explicitly supplied, versioned tradition pack. It is
not a universal Orthodox authority and does not infer that similar rank names
or output counts have identical meanings across traditions.

Authority records have three categories with deliberately different meanings:

- `source` identifies a publication or artifact from which facts may be drawn;
- `scoped_claim` states one reusable interpretation, its applicability, and
  the source records supporting it;
- `dated_witness` records the resolved service arrangement on one liturgical
  date, after multiple cycles and combination rules have interacted.

The separate `kind` field records evidentiary standing: `authoritative` or
`observed_behavior`. Category and standing are independent. An official dated
service order can be an authoritative `dated_witness`, while generated Digital
Chant Stand output is an `observed_behavior` `dated_witness`.

Rule claims and the selected observance's dated witnesses are combined in the
resulting decision, in that order with duplicates removed. A dated witness
supports the resulting arrangement only for its stated date; it does not by
itself establish the reusable claim or explain calculations from the fixed,
weekly, Paschal, or other cycles.

Calendar-system selection belongs to the pack, not to a jurisdiction branch in
Rust. The engine supports Gregorian, Revised Julian, and Julian fixed-date
projection independently from its Julian-based Orthodox Paschalion. This makes
Old Calendar calculation possible without asserting that every OCA or GOARCH
community uses the same fixed calendar.

The evaluator currently rejects ambiguity. Future models should represent
explicit supersession, local overrides, and review-required choices only when
real cases establish their semantics.

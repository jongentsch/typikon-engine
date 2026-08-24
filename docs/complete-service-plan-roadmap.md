# Complete service plan roadmap

## Implemented

- Pack plus civil date selects fixed and Paschal-offset observances.
- Vespers, Great Vespers, Matins, and Divine Liturgy are complete ordered
  structures with fixed and changeable components in all three packs.
- Divine Liturgy exposes Chrysostom and Basil forms and form-specific fixed
  material.
- Observances can define material inline and reuse it locally.
- Rank profiles define service-specific completeness requirements.
- Rules admit observance material by service/section/component.
- Plans contain resolved, omitted, and unresolved components and cannot report
  false completeness.
- Cross-pack tests select twelve major feasts from the date alone and inspect
  the complete service structures.

## Data work remaining

The twelve major-feast observances currently retain authoritative and dated
evidence but do not pretend that a whole dated webpage is reusable perennial
material. Their required components therefore compile as `unresolved`.

For each jurisdiction and feast, normalize and review:

- Vespers: kathisma appointment, Lord-I-Call stichera, entrance,
  prokeimenon, readings, Aposticha, dismissal hymns;
- Matins: apolytikia, kathismata, polyeleos, Gospel sequence, canon,
  exapostilaria, Praises, and dismissal material;
- Divine Liturgy: antiphons/Typika, entrance hymn, troparia, kontakia,
  Trisagion or substitute, Prokeimenon, Epistle, Alleluia, Gospel,
  Megalynarion, Communion hymn, and seasonal substitutions.

Each normalized item belongs in the feast's observance document (or its local
`common` map), cites authority, and gains a date-driven conformance assertion.
Completion is earned component by component.

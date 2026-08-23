# Complete service plan roadmap

## Target contract

A caller supplies a tradition pack and one target civil date. The engine
discovers the day's observances, calculates the fixed and Paschal cycles, and
returns every service that the pack can compile. The caller does not name a
feast or manually select service material.

```text
pack + civil date
        |
        v
calendar and observance selection
        |
        v
service structure + placement rules + observance appointments
        |
        v
service-keyed, evidence-bearing plans
```

## Implemented baseline

- `compile_date` derives the preceding civil evening for Vespers and returns a
  deterministic map keyed by service ID.
- Fixed dates and Paschal offsets select observances automatically.
- Services define sections and slots; rules either emit static cycle material
  or request a named observance appointment.
- Observances appoint typed resources by service and semantic role.
- Resources carry stable IDs, kind, role, official URL, and authority records.
- The compiled item contains the resolved resource metadata and the decision
  combines rule, observance, and resource provenance.
- GOARCH, OCA, and Antiochian packs each provide 36 resources: Vespers,
  Matins/Orthros, and Divine Liturgy for twelve major feasts.
- Cross-pack tests compile all 108 feast/service cases from the date alone.

The current feast resource granularity is `service-bundle` with the role
`complete-propers`. This is a complete pointer to the jurisdiction's published
proper material, not a claim that every constituent reading and hymn has been
normalized into separate engine records.

## Evidence-led expansion

The same appointment mechanism supports finer resources without another
ownership redesign. As authoritative data is transcribed, a bundle can be
replaced or supplemented by roles such as Gospel, Epistle, troparion,
kontakion, canon, and appointed stichera. Service schemas can then expose the
corresponding slots, while rules continue to express placement and collision
behavior. Each new normalized item must retain a retrievable authority record
and receive conformance coverage before it is treated as complete.

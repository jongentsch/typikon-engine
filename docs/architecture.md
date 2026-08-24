# Architecture

```text
pack + civil date
       |
       v
calendar projection and observance selection
       |
       v
service definition (complete ordered structure)
       + rank profile (required propers)
       + observance material (actual propers)
       + rules (appointment and combination)
       |
       v
typikon.plan/v0.2
```

The loader parses YAML, validates each document against JSON Schema,
deserializes typed records, and checks cross-references before evaluation.
Definition directories are recursive, so file paths can be organized for
humans while stable IDs remain the semantic keys.

## Compilation

1. Apply the service's `liturgical_day_offset` to the civil date.
2. Project the liturgical day into the pack's fixed calendar.
3. Calculate Orthodox Pascha, phase, weekday, and Octoechos tone.
4. Select observances by `date.fixed` or `date.paschal_offset`, unless the
   caller explicitly supplies observances.
5. Match rule predicates against the service, day, and observances.
6. Select a service form when a rule appoints one; otherwise use the service's
   default form.
7. Instantiate every service section and component. Fixed material comes
   directly from the service definition, including form-specific fixed
   material such as the Chrysostom or Basil Anaphora.
8. Admit static cycle material or the bound observance's material into each
   changeable component named by a matching rule.
9. Mark empty optional components `omitted`. Mark empty cardinality-one or
   rank-required components `unresolved`.
10. Return `complete` only when no required component is unresolved.

Rules do not own feast texts. `observance: true` on an emission means “take
this observance's material for this same service/section/component.” Missing
material is not fabricated and is not a compilation exception; it is visible
in the plan as an unresolved requirement.

The evaluator is deterministic: definitions use ordered maps, service order is
preserved, selected observances are sorted, and decisions are numbered in
evaluation order.

## Boundaries

`TraditionResource` abstracts pack bytes. The filesystem implementation is
confined to the pack root; the memory implementation supports FFI and embedded
callers. `typikon-core` performs no filesystem or network access.

Authority records remain evidence, not runtime appointments. Dated witnesses
can test a plan for that date but never become recurring material merely
because a later civil date selects the same feast.

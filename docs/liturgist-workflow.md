# Liturgist workflow: add a saint

Suppose a jurisdiction recognizes Saint Example as a major saint, but that
rank does not exist yet.

## 1. Define the rank once

Create `ranks/major-saint.yaml`. State the authoritative rubric source and the
changeable components required in every applicable service:

```yaml
schema: typikon.rank/v0.1
id: major-saint
name: Major saint
authority: [jurisdiction-typikon]
services:
  divine_liturgy:
    required:
      - { section: hymns, component: troparia }
      - { section: hymns, component: kontakia }
      - { section: readings, component: prokeimenon }
      - { section: readings, component: epistle }
      - { section: readings, component: alleluia }
      - { section: readings, component: gospel }
  matins:
    required:
      - { section: god_is_lord, component: apolytikia }
      - { section: canon, component: canon }
```

The loader verifies that every path names a real changeable component.

## 2. Author one saint document

Write shared hymns once under `common`, then place them where each service uses
them. Inline service-specific readings directly.

```yaml
schema: typikon.observance/v0.4
id: example-the-wonderworker
name: Saint Example the Wonderworker
date:
  fixed: { month: 7, day: 10 }
rank: major-saint
authority: [official-july-menaion]

common:
  apolytikion:
    kind: hymn
    role: apolytikion
    title: Apolytikion of Saint Example
    text: ...
    authority: [official-july-menaion]

services:
  matins:
    god_is_lord:
      apolytikia:
        use: common.apolytikion
  divine_liturgy:
    hymns:
      troparia:
        use: common.apolytikion
    readings:
      epistle:
        kind: scripture
        role: epistle
        title: Epistle for Saint Example
        citation: Hebrews 7:26-8:2
        authority: [official-july-menaion]
```

There is no second resource document and no copied global ID.

## 3. Use an existing rank rule

The pack's general major-saint rules should already say which observance
components are admitted into each service. A liturgist normally does not edit a
rule to add another saint.

## 4. Compile the date and finish the checklist

Run `compile-date`. The plan includes the full fixed service immediately. Each
rank-required component is either resolved from the saint document or marked
`unresolved`. Add and review material until the plan becomes `complete`, then
retain a published service order as a dated conformance witness.

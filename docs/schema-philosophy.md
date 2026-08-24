# Schema philosophy

The authoring model follows the way a liturgist divides the work.

## Service owns structure

A service definition is the whole ordered service. Components are explicitly
`fixed` or `changeable`. Fixed components carry their material or citation in
the service document. Changeable components declare cardinality and the roles
they accept. Service forms model real variants such as the Divine Liturgies of
Saint John Chrysostom and Saint Basil, not feast-specific pseudo-services.

## Observance owns propers

An observance owns:

- its uniform `date` expression;
- its rank;
- reusable local material under `common`;
- its contribution to each service, section, and component;
- evidence supporting those claims.

Material can be inline or `use: common.<id>`. Separate global documents are
reserved for genuinely shared books or corpora, not every saint's hymn.

## Rank owns completeness

A rank profile is a pack-maintained rubric contract. Its service-specific
requirements tell the compiler—and the editor—what a saint of that rank still
needs. A new rank is authored once, reviewed against authority, and then reused
by observances.

## Rules own appointment and combination

Rules decide when cycle material and observance material enter a service
component. They may also select a service form. They do not contain a duplicate
copy of an observance's text and do not name dated output bundles.

## Evidence never becomes recurrence by accident

Authority records distinguish retrievable sources, reusable scoped claims, and
dated witnesses. A dated witness may verify a compiled date; it is not a
perennial appointment. Plans remain reviewable when authoritative evidence has
not yet been normalized at component granularity.

`observance.properties` remains a narrow escape hatch for evidence-led
experiments. A concept graduates into a typed schema field only after real pack
data establishes stable semantics.

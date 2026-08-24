use std::collections::BTreeMap;

use serde_json::json;
use typikon_loader::{LoaderError, MemoryResource, SchemaKind, load_pack, validate_value};

#[test]
fn malformed_yaml_names_its_source() {
    let error = load_pack(&MemoryResource::from_text([(
        "pack.yaml".to_owned(),
        "[".to_owned(),
    )]))
    .unwrap_err();
    assert!(matches!(error, LoaderError::MalformedYaml { .. }));
    assert!(error.to_string().contains("pack.yaml"));
}

#[test]
fn unsupported_schema_version_fails_before_deserialization() {
    let resource = MemoryResource::from_text([(
        "pack.yaml".to_owned(),
        "schema: typikon.pack/v9\nid: test\nname: Test\nversion: 0.1.0\n".to_owned(),
    )]);
    let error = load_pack(&resource).unwrap_err();
    assert!(matches!(error, LoaderError::Schema { .. }));
    assert!(error.to_string().contains("typikon.pack/v0.3"));
}

#[test]
fn observance_dates_share_one_uniform_date_location() {
    for date in [
        json!({ "fixed": { "month": 12, "day": 25 } }),
        json!({ "paschal_offset": -7 }),
    ] {
        let observance = json!({
            "schema": "typikon.observance/v0.4", "id": "test", "name": "Test",
            "date": date, "rank": "major-feast"
        });
        validate_value(SchemaKind::Observance, "test.yaml", &observance).unwrap();
    }
    let invalid = json!({
        "schema": "typikon.observance/v0.4", "id": "test", "name": "Test",
        "date": { "fixed": { "month": 12, "day": 25 }, "paschal_offset": 0 },
        "rank": "major-feast"
    });
    assert!(validate_value(SchemaKind::Observance, "test.yaml", &invalid).is_err());
}

#[test]
fn rule_emission_requires_exactly_one_material_source() {
    let neither = json!({
        "schema": "typikon.rule/v0.3", "id": "test",
        "when": { "service": "vespers", "observance": { "rank": "feast" } },
        "emit": [{ "section": "propers", "component": "hymn" }]
    });
    assert!(validate_value(SchemaKind::Rule, "test.yaml", &neither).is_err());

    let observance = json!({
        "schema": "typikon.rule/v0.3", "id": "test",
        "when": { "service": "vespers", "observance": { "rank": "feast" } },
        "emit": [{ "section": "propers", "component": "hymn", "observance": true }]
    });
    validate_value(SchemaKind::Rule, "test.yaml", &observance).unwrap();

    let mut both = observance;
    both["emit"][0]["material"] = json!({ "kind": "hymn", "title": "Hymn" });
    assert!(validate_value(SchemaKind::Rule, "test.yaml", &both).is_err());
}

fn valid_files() -> BTreeMap<String, Vec<u8>> {
    [
        (
            "pack.yaml",
            r"schema: typikon.pack/v0.3
id: test
name: Test
version: 0.1.0
calendar:
  fixed: revised_julian
  paschalion: orthodox_julian
  tone_cycle:
    system: octoechos
    tones: [tone_1, tone_2, tone_3, tone_4, tone_5, tone_6, tone_7, tone_8]
definitions:
  services: services/
  observances: observances/
  ranks: ranks/
  rules: rules/
  authorities: authorities/
",
        ),
        (
            "authorities/source.yaml",
            r"schema: typikon.authority/v0.1
id: source
title: Source
category: source
kind: authoritative
reference: { url: https://example.test/source }
",
        ),
        (
            "services/vespers.yaml",
            r"schema: typikon.service/v0.2
id: vespers
name: Vespers
liturgical_day_offset: 1
authority: [source]
sections:
  - id: psalms
    name: Psalms
    components:
      - id: opening
        name: Opening
        kind: fixed
        material: { kind: fixed_text, title: Opening }
      - id: verses
        name: Verses
        kind: changeable
        cardinality: many
        accepts: [hymn]
",
        ),
        (
            "ranks/major.yaml",
            r"schema: typikon.rank/v0.1
id: major-feast
name: Major feast
authority: [source]
services:
  vespers:
    required:
      - { section: psalms, component: verses }
",
        ),
        (
            "observances/feasts/major/nativity.yaml",
            r"schema: typikon.observance/v0.4
id: nativity-christ
name: Nativity of Christ
date:
  fixed: { month: 12, day: 25 }
rank: major-feast
common:
  hymn: { kind: sticheron, role: hymn, title: Nativity hymn }
services:
  vespers:
    psalms:
      verses: { use: common.hymn }
",
        ),
        (
            "rules/vespers.yaml",
            r"schema: typikon.rule/v0.3
id: vespers
when:
  service: vespers
  observance: { rank: major-feast }
emit:
  - { section: psalms, component: verses, observance: true }
",
        ),
    ]
    .map(|(path, contents)| (path.to_owned(), contents.as_bytes().to_vec()))
    .into_iter()
    .collect()
}

#[test]
fn recursive_taxonomy_and_local_material_references_load() {
    let pack = load_pack(&MemoryResource::new(valid_files())).unwrap();
    assert_eq!(
        pack.observances["nativity-christ"].source,
        "observances/feasts/major/nativity.yaml"
    );
    assert!(pack.ranks.contains_key("major-feast"));
}

#[test]
fn unknown_component_reference_names_the_rule_and_path() {
    let mut files = valid_files();
    files.insert(
        "rules/vespers.yaml".to_owned(),
        br"schema: typikon.rule/v0.3
id: bad-rule
when: { service: vespers }
emit:
  - section: psalms
    component: missing
    material: { kind: hymn, title: Hymn }
"
        .to_vec(),
    );
    let error = load_pack(&MemoryResource::new(files)).unwrap_err();
    assert!(matches!(error, LoaderError::UnknownReference { .. }));
    assert!(error.to_string().contains("bad-rule"));
    assert!(error.to_string().contains("psalms:missing"));
}

#[test]
fn observance_cannot_write_into_a_fixed_component() {
    let mut files = valid_files();
    let observance = String::from_utf8(
        files
            .remove("observances/feasts/major/nativity.yaml")
            .unwrap(),
    )
    .unwrap()
    .replace(
        "verses: { use: common.hymn }",
        "opening: { use: common.hymn }",
    );
    files.insert(
        "observances/feasts/major/nativity.yaml".to_owned(),
        observance.into_bytes(),
    );
    let error = load_pack(&MemoryResource::new(files)).unwrap_err();
    assert!(matches!(error, LoaderError::Schema { .. }));
    assert!(error.to_string().contains("fixed component"));
}

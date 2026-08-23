use std::collections::BTreeMap;

use typikon_loader::{LoaderError, MemoryResource, load_pack};

#[test]
fn malformed_yaml_names_its_source() {
    let resource = MemoryResource::from_text([("pack.yaml".to_owned(), "[".to_owned())]);
    let error = load_pack(&resource).unwrap_err();
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
    assert!(error.to_string().contains("typikon.pack/v0.1"));
}

#[test]
fn observances_load_recursively_from_taxonomy_directories() {
    let files = BTreeMap::from([
        (
            "pack.yaml".to_owned(),
            br"schema: typikon.pack/v0.1
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
  rules: rules/
  authorities: authorities/
"
            .to_vec(),
        ),
        (
            "services/vespers.yaml".to_owned(),
            br"schema: typikon.service/v0.1
id: vespers
name: Vespers
liturgical_day_offset: 1
sections:
  - id: psalms
    slots:
      - id: verses
        cardinality: many
"
            .to_vec(),
        ),
        (
            "observances/feasts/major/nativity-christ.yaml".to_owned(),
            br"schema: typikon.observance/v0.1
id: nativity-christ
name: Nativity of Christ
date:
  fixed:
    month: 12
    day: 25
rank: major-feast
"
            .to_vec(),
        ),
        (
            "rules/vespers.yaml".to_owned(),
            br"schema: typikon.rule/v0.1
id: vespers
when:
  service: vespers
emit:
  - section: psalms
    slot: verses
    material:
      source: test
"
            .to_vec(),
        ),
    ]);

    let pack = load_pack(&MemoryResource::new(files)).expect("nested observance should load");
    let observance = pack
        .observances
        .get("nativity-christ")
        .expect("observance should be indexed by its stable ID");

    assert_eq!(
        observance.source,
        "observances/feasts/major/nativity-christ.yaml"
    );
}

#[test]
fn an_unknown_emission_slot_names_the_rule_and_slot() {
    let files = BTreeMap::from([
        (
            "pack.yaml".to_owned(),
            br"schema: typikon.pack/v0.1
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
  rules: rules/
  authorities: authorities/
"
            .to_vec(),
        ),
        (
            "services/vespers.yaml".to_owned(),
            br"schema: typikon.service/v0.1
id: vespers
name: Vespers
liturgical_day_offset: 1
sections:
  - id: psalms
    slots:
      - id: verses
        cardinality: many
"
            .to_vec(),
        ),
        (
            "rules/bad.yaml".to_owned(),
            br"schema: typikon.rule/v0.1
id: bad-rule
when:
  service: vespers
emit:
  - section: psalms
    slot: missing
    material:
      source: test
"
            .to_vec(),
        ),
    ]);
    let error = load_pack(&MemoryResource::new(files)).unwrap_err();
    assert!(matches!(error, LoaderError::UnknownReference { .. }));
    let message = error.to_string();
    assert!(message.contains("bad-rule"));
    assert!(message.contains("psalms:missing"));
}

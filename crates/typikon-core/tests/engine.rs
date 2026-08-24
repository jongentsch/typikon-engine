use std::collections::BTreeMap;

use typikon_core::{Engine, EngineError, InteropError};
use typikon_loader::{MemoryResource, SchemaKind, Sourced, load_pack, validate_value};
use typikon_schema::{
    CompileServiceRequest, PlanComponentStatus, PlanStatus, REQUEST_SCHEMA, ServiceFormDefinition,
};

#[allow(clippy::too_many_lines, clippy::needless_raw_string_hashes)]
fn synthetic_pack() -> typikon_loader::LoadedPack {
    let files = BTreeMap::from(
        [
            (
                "pack.yaml",
                r#"schema: typikon.pack/v0.3
id: synthetic
name: Synthetic pack
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
"#,
            ),
            (
                "authorities/source.yaml",
                r#"schema: typikon.authority/v0.1
id: synthetic-source
title: Synthetic source
category: source
kind: authoritative
reference:
  url: https://example.test/source
"#,
            ),
            (
                "services/vespers.yaml",
                r#"schema: typikon.service/v0.2
id: great_vespers
name: Great Vespers
liturgical_day_offset: 1
authority: [synthetic-source]
sections:
  - id: opening
    name: Opening
    components:
      - id: opening_blessing
        name: Opening blessing
        kind: fixed
        material:
          kind: fixed_text
          title: Opening blessing
          text: Blessed is our God
  - id: lord_i_call
    name: Lord, I Call
    components:
      - id: stichera
        name: Stichera
        kind: changeable
        cardinality: many
        accepts: [resurrectional, saint]
      - id: glory
        name: Glory
        kind: changeable
        cardinality: one
        accepts: [doxastikon]
      - id: both_now
        name: Both now
        kind: changeable
        cardinality: optional
        accepts: [theotokion]
"#,
            ),
            (
                "ranks/six.yaml",
                r#"schema: typikon.rank/v0.1
id: six-stichera
name: Six stichera
authority: [synthetic-source]
services:
  great_vespers:
    required:
      - section: lord_i_call
        component: stichera
      - section: lord_i_call
        component: glory
"#,
            ),
            (
                "observances/saint.yaml",
                r#"schema: typikon.observance/v0.4
id: primary-context
name: Primary context
date:
  fixed: { month: 7, day: 26 }
rank: six-stichera
authority: [synthetic-source]
common:
  doxastikon:
    kind: sticheron
    role: doxastikon
    title: Doxastikon of the saint
    text: Glory proper
services:
  great_vespers:
    lord_i_call:
      stichera:
        kind: hymn_set
        role: saint
        title: Stichera of the saint
      glory:
        use: common.doxastikon
properties:
  has_glory: true
"#,
            ),
            (
                "rules/ordinary.yaml",
                r#"schema: typikon.rule/v0.3
id: ordinary-rule
when:
  service: great_vespers
  day:
    weekday: sunday
    phase: ordinary
  observance:
    rank: six-stichera
emit:
  - section: lord_i_call
    component: stichera
    material:
      kind: hymn_set
      role: resurrectional
      title: Octoechos stichera
      attributes:
        tone: $day.tone
    count: 6
  - section: lord_i_call
    component: stichera
    observance: true
    count: 4
  - section: lord_i_call
    component: glory
    observance: true
  - section: lord_i_call
    component: both_now
    material:
      kind: theotokion
      role: theotokion
      title: Cycle theotokion
authority: [synthetic-source]
"#,
            ),
        ]
        .map(|(path, contents)| (path.to_owned(), contents.as_bytes().to_vec())),
    );
    load_pack(&MemoryResource::new(files)).unwrap()
}

fn engine() -> Engine {
    Engine::new(synthetic_pack())
}

fn request(date: &str) -> CompileServiceRequest {
    CompileServiceRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        civil_date: date.to_owned(),
        service: "great_vespers".to_owned(),
        tone: None,
        phase: None,
        observances: Vec::new(),
    }
}

#[test]
fn plan_contains_fixed_structure_and_observance_owned_material() {
    let plan = engine().compile_service(request("2026-07-25")).unwrap();
    validate_value(
        SchemaKind::Plan,
        "plan",
        &serde_json::to_value(&plan).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.status, PlanStatus::Complete);
    assert_eq!(plan.day.liturgical_date, "2026-07-26");
    assert_eq!(plan.observances[0].id, "primary-context");

    let fixed = &plan.sections[0].components[0];
    assert_eq!(fixed.status, PlanComponentStatus::Resolved);
    assert_eq!(fixed.materials[0].decision, None);
    assert_eq!(fixed.materials[0].material["text"], "Blessed is our God");

    let stichera = &plan.sections[1].components[0];
    assert_eq!(stichera.materials.len(), 2);
    assert_eq!(stichera.materials[0].count, Some(6));
    assert_eq!(
        stichera.materials[0].material["attributes"]["tone"],
        "tone_7"
    );
    assert_eq!(
        stichera.materials[1].material["title"],
        "Stichera of the saint"
    );
    assert_eq!(
        plan.sections[1].components[1].materials[0].material["text"],
        "Glory proper"
    );
}

#[test]
fn missing_rank_material_is_a_reviewable_plan_not_false_completeness() {
    let mut pack = synthetic_pack();
    pack.observances
        .get_mut("primary-context")
        .unwrap()
        .value
        .services
        .get_mut("great_vespers")
        .unwrap()
        .get_mut("lord_i_call")
        .unwrap()
        .remove("glory");
    let plan = Engine::new(pack)
        .compile_service(request("2026-07-25"))
        .unwrap();
    assert_eq!(plan.status, PlanStatus::RequiresReview);
    assert_eq!(
        plan.sections[1].components[1].status,
        PlanComponentStatus::Unresolved
    );
    assert!(plan.sections[1].components[1].materials.is_empty());
}

#[test]
fn a_rule_can_select_form_specific_fixed_material() {
    let mut pack = synthetic_pack();
    let service = &mut pack.services.get_mut("great_vespers").unwrap().value;
    service.default_form = Some("chrysostom".to_owned());
    service.forms = vec![
        ServiceFormDefinition {
            id: "chrysostom".to_owned(),
            name: "Chrysostom".to_owned(),
            authority: vec!["synthetic-source".to_owned()],
        },
        ServiceFormDefinition {
            id: "basil".to_owned(),
            name: "Basil".to_owned(),
            authority: vec!["synthetic-source".to_owned()],
        },
    ];
    service.sections[0].components[0].form_material.insert(
        "basil".to_owned(),
        serde_json::from_value(
            serde_json::json!({ "kind": "fixed_text", "title": "Basil form blessing" }),
        )
        .unwrap(),
    );

    let mut form_rule = pack.rules["ordinary-rule"].clone();
    form_rule.value.id = "select-basil".to_owned();
    form_rule.value.emit.clear();
    form_rule.value.select_form = Some("basil".to_owned());
    pack.rules.insert(
        form_rule.value.id.clone(),
        Sourced {
            source: "test:form".to_owned(),
            value: form_rule.value,
        },
    );

    let plan = Engine::new(pack)
        .compile_service(request("2026-07-25"))
        .unwrap();
    assert_eq!(plan.form.as_deref(), Some("basil"));
    assert_eq!(
        plan.sections[0].components[0].materials[0].material["title"],
        "Basil form blessing"
    );
}

#[test]
fn compile_date_selects_the_observance_without_a_feast_parameter() {
    let plans = engine().compile_date("2026-07-26").unwrap();
    assert_eq!(
        plans.keys().map(String::as_str).collect::<Vec<_>>(),
        ["great_vespers"]
    );
    assert_eq!(plans["great_vespers"].request.civil_date, "2026-07-25");
    assert_eq!(plans["great_vespers"].observances[0].id, "primary-context");
}

#[test]
fn serialized_boundary_is_valid_and_deterministic() {
    let request = serde_json::json!({
        "schema": REQUEST_SCHEMA,
        "civil_date": "2026-07-25",
        "service": "great_vespers"
    })
    .to_string();
    let engine = engine();
    let first = engine.compile_service_json(&request).unwrap();
    assert_eq!(first, engine.compile_service_json(&request).unwrap());
    validate_value(
        SchemaKind::Plan,
        "serialized plan",
        &serde_json::from_str(&first).unwrap(),
    )
    .unwrap();

    let invalid = serde_json::json!({
        "schema": REQUEST_SCHEMA, "civil_date": "2026-07-25",
        "service": "great_vespers", "unexpected": true
    })
    .to_string();
    assert!(matches!(
        engine.compile_service_json(&invalid),
        Err(InteropError::InvalidRequest(_))
    ));
}

#[test]
fn request_validation_rejects_bad_context_and_dates() {
    let mut bad = request("2026-07-25");
    bad.tone = Some("Tone seven".to_owned());
    assert!(matches!(
        engine().compile_service(bad),
        Err(EngineError::InvalidContextValue { .. })
    ));
    assert!(matches!(
        engine().compile_service(request("2026-7-25")),
        Err(EngineError::InvalidCivilDate { .. })
    ));
}

use std::collections::BTreeMap;

use typikon_core::{Engine, EngineError, InteropError};
use typikon_loader::{MemoryResource, SchemaKind, Sourced, load_pack, validate_value};
use typikon_schema::{
    CompileServiceRequest, FixedCalendar, ObservanceDate, ObservancePredicate, OneOrMany,
    REQUEST_SCHEMA, RulePredicate,
};

// Complete inline YAML documents keep the self-contained fixture reviewable.
#[allow(clippy::too_many_lines)]
fn synthetic_pack() -> typikon_loader::LoadedPack {
    let files = BTreeMap::from([
        (
            "pack.yaml".to_owned(),
            r"schema: typikon.pack/v0.1
id: synthetic
name: Synthetic engine test pack
version: 0.1.0
calendar:
  fixed: revised_julian
  paschalion: orthodox_julian
  tone_cycle:
    system: octoechos
    tones:
      - tone_1
      - tone_2
      - tone_3
      - tone_4
      - tone_5
      - tone_6
      - tone_7
      - tone_8
definitions:
  services: services/
  observances: observances/
  rules: rules/
  authorities: authorities/
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "services/vespers.yaml".to_owned(),
            r"schema: typikon.service/v0.1
id: great_vespers
name: Great Vespers
liturgical_day_offset: 1
sections:
  - id: lord_i_call
    slots:
      - id: stichera
        cardinality: many
      - id: glory
        cardinality: optional
      - id: both_now
        cardinality: optional
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "observances/primary.yaml".to_owned(),
            r"schema: typikon.observance/v0.2
id: primary-context
name: Primary context
date:
  fixed:
    month: 7
    day: 26
rank: six-stichera
authority:
  - synthetic-observation
properties:
  has_glory: true
  glory_tone: tone_6
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "observances/blocking.yaml".to_owned(),
            r"schema: typikon.observance/v0.2
id: blocking-context
name: Blocking context
date:
  fixed:
    month: 1
    day: 1
rank: blocking
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "rules/ordinary.yaml".to_owned(),
            r"schema: typikon.rule/v0.1
id: ordinary-rule
when:
  service: great_vespers
  day:
    weekday: sunday
    phase: ordinary
  observance:
    rank: six-stichera
    properties:
      has_glory: true
emit:
  - section: lord_i_call
    slot: stichera
    material:
      source: cycle
      role: primary
      tone: $day.tone
    count: 6
  - section: lord_i_call
    slot: stichera
    material:
      source: observance
      role: secondary
      observance: $observance.id
    count: 4
  - section: lord_i_call
    slot: glory
    material:
      source: observance
      role: doxastikon
  - section: lord_i_call
    slot: both_now
    material:
      source: cycle
      role: theotokion
      tone: $observance.properties.glory_tone
authority:
  - synthetic-authority
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "authorities/source.yaml".to_owned(),
            r"schema: typikon.authority/v0.1
id: synthetic-source
title: Synthetic source publication
category: source
kind: authoritative
reference:
  url: https://example.test/source
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "authorities/claim.yaml".to_owned(),
            r"schema: typikon.authority/v0.1
id: synthetic-authority
title: Synthetic scoped claim
category: scoped_claim
kind: authoritative
sources:
  - synthetic-source
claim: The synthetic arrangement applies in the fixture context.
"
            .as_bytes()
            .to_vec(),
        ),
        (
            "authorities/observation.yaml".to_owned(),
            r"schema: typikon.authority/v0.1
id: synthetic-observation
title: Synthetic dated observation
category: dated_witness
kind: observed_behavior
locator:
  liturgical_date: 2026-07-26
reference:
  url: https://example.test/witness
"
            .as_bytes()
            .to_vec(),
        ),
    ]);
    load_pack(&MemoryResource::new(files)).unwrap()
}

fn engine() -> Engine {
    Engine::new(synthetic_pack())
}

fn request(date: &str, observances: &[&str]) -> CompileServiceRequest {
    CompileServiceRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        civil_date: date.to_owned(),
        service: "great_vespers".to_owned(),
        tone: None,
        phase: None,
        observances: observances
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

#[test]
fn synthetic_pack_compiles_a_schema_valid_plan() {
    let plan = engine()
        .compile_service(request("2026-07-25", &[]))
        .unwrap();
    let value = serde_json::to_value(&plan).unwrap();
    validate_value(SchemaKind::Plan, "synthetic plan", &value).unwrap();

    assert_eq!(plan.pack.id, "synthetic");
    assert_eq!(plan.day.liturgical_date, "2026-07-26");
    assert_eq!(plan.day.weekday, "sunday");
    assert_eq!(plan.day.tone.as_deref(), Some("tone_7"));
    assert_eq!(plan.day.phase, "ordinary");
    assert_eq!(plan.day.pascha, "2026-04-12");
    assert_eq!(plan.derivations[5].component, "tone");
    assert!(plan.request.observances.is_empty());
    assert_eq!(plan.sections[0].items[0].count, Some(6));
    assert_eq!(plan.sections[0].items[1].count, Some(4));
    assert_eq!(plan.decisions[0].rule, "ordinary-rule");
    assert_eq!(
        plan.decisions[0].authority,
        ["synthetic-authority", "synthetic-observation"]
    );
}

#[test]
fn serialized_boundary_validates_versions_and_is_deterministic() {
    let request_json = serde_json::json!({
        "schema": REQUEST_SCHEMA,
        "civil_date": "2026-07-25",
        "service": "great_vespers"
    })
    .to_string();
    let engine = engine();
    let first = engine.compile_service_json(&request_json).unwrap();
    let second = engine.compile_service_json(&request_json).unwrap();
    assert_eq!(first, second);

    let plan: serde_json::Value = serde_json::from_str(&first).unwrap();
    validate_value(SchemaKind::Plan, "serialized plan", &plan).unwrap();
    assert_eq!(plan["request"]["observances"], serde_json::json!([]));
    assert_eq!(plan["observances"][0]["id"], "primary-context");

    let invalid = serde_json::json!({
        "schema": REQUEST_SCHEMA,
        "civil_date": "2026-07-25",
        "service": "great_vespers",
        "unexpected": true
    })
    .to_string();
    assert!(matches!(
        engine.compile_service_json(&invalid),
        Err(InteropError::InvalidRequest(_))
    ));

    let mut unsupported = request("2026-07-25", &[]);
    unsupported.schema = "typikon.request/v9".to_owned();
    assert!(matches!(
        engine.compile_service(unsupported),
        Err(EngineError::UnsupportedRequestSchema(_))
    ));
}

#[test]
fn compilation_is_byte_for_byte_deterministic() {
    let engine = engine();
    let first = engine.compile_service(request("2026-07-25", &[])).unwrap();
    let second = engine.compile_service(request("2026-07-25", &[])).unwrap();

    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn date_level_compilation_derives_service_start_dates_without_caller_context() {
    let plans = engine().compile_date("2026-07-26").unwrap();
    assert_eq!(
        plans.keys().map(String::as_str).collect::<Vec<_>>(),
        ["great_vespers"]
    );
    let plan = &plans["great_vespers"];
    assert_eq!(plan.request.civil_date, "2026-07-25");
    assert!(plan.request.observances.is_empty());
    assert_eq!(plan.day.liturgical_date, "2026-07-26");
    assert_eq!(plan.observances[0].id, "primary-context");
}

#[test]
fn conflicting_exclusive_emissions_report_ambiguity() {
    let mut pack = synthetic_pack();
    let mut duplicate = pack.rules["ordinary-rule"].clone();
    duplicate.value.id = "second-rule".to_owned();
    pack.rules.insert(
        duplicate.value.id.clone(),
        Sourced {
            source: "test:synthetic-conflict".to_owned(),
            value: duplicate.value,
        },
    );

    let error = Engine::new(pack)
        .compile_service(request("2026-07-25", &[]))
        .unwrap_err();
    assert!(matches!(error, EngineError::AmbiguousSlot { .. }));
    assert!(error.to_string().contains("lord_i_call:glory"));
}

#[test]
fn invalid_context_values_cannot_escape_the_plan_contract() {
    let mut invalid_tone = request("2026-07-25", &[]);
    invalid_tone.tone = Some("Tone seven".to_owned());
    let error = engine().compile_service(invalid_tone).unwrap_err();
    assert!(matches!(
        error,
        EngineError::InvalidContextValue { field: "tone", .. }
    ));

    let mut wrong_tone = request("2026-07-25", &[]);
    wrong_tone.tone = Some("tone_6".to_owned());
    let error = engine().compile_service(wrong_tone).unwrap_err();
    assert!(matches!(error, EngineError::ToneMismatch { .. }));

    let mut wrong_phase = request("2026-07-25", &[]);
    wrong_phase.phase = Some("triodion".to_owned());
    let error = engine().compile_service(wrong_phase).unwrap_err();
    assert!(matches!(error, EngineError::PhaseMismatch { .. }));

    let error = engine()
        .compile_service(request("2026-7-25", &[]))
        .unwrap_err();
    assert!(matches!(error, EngineError::InvalidCivilDate { .. }));
}

#[test]
fn old_calendar_projection_selects_the_fixed_observance() {
    let mut pack = synthetic_pack();
    pack.pack.value.calendar.fixed = FixedCalendar::Julian;
    pack.rules
        .get_mut("ordinary-rule")
        .unwrap()
        .value
        .when
        .day
        .as_mut()
        .unwrap()
        .phase = Some(OneOrMany::One("triodion".to_owned()));
    let date = &mut pack
        .observances
        .get_mut("primary-context")
        .unwrap()
        .value
        .date;
    let ObservanceDate::Fixed { fixed } = date else {
        panic!("synthetic primary observance should have a fixed date");
    };
    fixed.month = 2;
    fixed.day = 29;

    let plan = Engine::new(pack)
        .compile_service(request("2100-03-13", &[]))
        .unwrap();
    let value = serde_json::to_value(&plan).unwrap();
    validate_value(SchemaKind::Plan, "Old Calendar leap-day plan", &value).unwrap();

    assert_eq!(plan.day.liturgical_date, "2100-03-14");
    assert_eq!(plan.day.fixed_date, "2100-02-29");
    assert_eq!(plan.day.fixed_calendar, FixedCalendar::Julian);
    assert_eq!(plan.observances[0].id, "primary-context");
    assert_eq!(
        plan.observances[0].selection_derivation.as_deref(),
        Some("derivation-0002")
    );
}

#[test]
fn bright_week_plan_exposes_suspended_tone_without_fabricating_one() {
    let mut pack = synthetic_pack();
    let rule = &mut pack.rules.get_mut("ordinary-rule").unwrap().value;
    rule.when.day.as_mut().unwrap().phase = Some(OneOrMany::One("pentecostarion".to_owned()));
    for emission in &mut rule.emit {
        if emission
            .material
            .get("tone")
            .and_then(serde_json::Value::as_str)
            == Some("$day.tone")
        {
            emission.material.insert(
                "tone".to_owned(),
                serde_json::Value::String("paschal".to_owned()),
            );
        }
    }

    let plan = Engine::new(pack)
        .compile_service(request("2026-04-11", &["primary-context"]))
        .unwrap();
    let value = serde_json::to_value(&plan).unwrap();
    validate_value(SchemaKind::Plan, "Bright Week plan", &value).unwrap();

    assert_eq!(plan.day.phase, "pentecostarion");
    assert_eq!(plan.day.tone, None);
    assert_eq!(plan.derivations[5].output, serde_json::Value::Null);
    assert_eq!(
        plan.derivations[5].method,
        "octoechos_suspended_bright_week"
    );
}

#[test]
fn unless_observance_checks_the_whole_selected_context() {
    let mut pack = synthetic_pack();
    pack.rules.get_mut("ordinary-rule").unwrap().value.unless = Some(RulePredicate {
        observance: Some(ObservancePredicate {
            rank: Some(OneOrMany::One("blocking".to_owned())),
            ..Default::default()
        }),
        ..Default::default()
    });

    let error = Engine::new(pack)
        .compile_service(request(
            "2026-07-25",
            &["primary-context", "blocking-context"],
        ))
        .unwrap_err();
    assert!(matches!(error, EngineError::NoMatchingRules { .. }));
}

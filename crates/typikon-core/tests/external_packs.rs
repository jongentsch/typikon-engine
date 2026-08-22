#![cfg(feature = "conformance")]

use std::path::{Path, PathBuf};

use serde_json::Value;
use typikon_core::Engine;
use typikon_loader::{DirectoryResource, SchemaKind, load_pack, validate_value};
use typikon_schema::CompileServiceRequest;

fn sibling_pack(name: &str) -> PathBuf {
    let environment_variable = match name {
        "typikon-goarch" => "TYPIKON_GOARCH_PACK",
        "typikon-oca" => "TYPIKON_OCA_PACK",
        _ => unreachable!("test names are fixed"),
    };
    if let Some(path) = std::env::var_os(environment_variable) {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(name)
}

fn engine(name: &str) -> Engine {
    let resource = DirectoryResource::new(sibling_pack(name)).unwrap();
    Engine::new(load_pack(&resource).unwrap())
}

fn request(date: &str, tone: &str, observances: &[&str]) -> CompileServiceRequest {
    CompileServiceRequest {
        civil_date: date.to_owned(),
        service: "great_vespers".to_owned(),
        tone: tone.to_owned(),
        phase: "ordinary".to_owned(),
        observances: observances
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

fn assert_golden(plan: &typikon_schema::Plan, expected: &str) {
    let actual = serde_json::to_value(plan).unwrap();
    let expected: Value = serde_json::from_str(expected).unwrap();
    assert_eq!(actual, expected);
    validate_value(SchemaKind::Plan, "test plan", &actual).unwrap();
}

#[test]
fn goarch_dcs_paraskevi_case_matches_observed_plan() {
    let plan = engine("typikon-goarch")
        .compile_service(request("2026-07-25", "grave", &[]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/goarch-2026-07-25-vespers.json"));
    assert_eq!(plan.day.liturgical_date, "2026-07-26");
    assert_eq!(
        plan.decisions[0].authority,
        ["goarch-dcs-2026-07-25-vespers"]
    );
}

#[test]
fn oca_lesser_saint_context_matches_published_seven_plus_three_case() {
    let plan = engine("typikon-oca")
        .compile_service(request("2026-08-22", "tone_3", &["lesser-saint-context"]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/oca-lesser-saint-vespers.json"));
}

#[test]
fn oca_six_stichera_context_matches_published_six_plus_four_case() {
    let plan = engine("typikon-oca")
        .compile_service(request(
            "2026-08-22",
            "tone_3",
            &["six-stichera-saint-context"],
        ))
        .unwrap();

    assert_golden(&plan, include_str!("golden/oca-six-stichera-vespers.json"));
}

#[test]
fn both_traditions_use_the_same_engine_vocabulary() {
    let goarch = engine("typikon-goarch")
        .compile_service(request("2026-07-25", "grave", &[]))
        .unwrap();
    let oca = engine("typikon-oca")
        .compile_service(request(
            "2026-08-22",
            "tone_3",
            &["six-stichera-saint-context"],
        ))
        .unwrap();

    assert_eq!(goarch.sections[0].id, oca.sections[0].id);
    let goarch_slots = goarch.sections[0]
        .items
        .iter()
        .map(|item| item.slot.as_str())
        .collect::<Vec<_>>();
    let oca_slots = oca.sections[0]
        .items
        .iter()
        .map(|item| item.slot.as_str())
        .collect::<Vec<_>>();
    assert_eq!(goarch_slots, oca_slots);
}

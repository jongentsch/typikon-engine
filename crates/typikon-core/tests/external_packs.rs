#![cfg(feature = "conformance")]

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde_json::Value;
use typikon_core::Engine;
use typikon_loader::{DirectoryResource, SchemaKind, load_pack, validate_value};
use typikon_schema::{CompileServiceRequest, REQUEST_SCHEMA};

fn sibling_pack(name: &str) -> PathBuf {
    let environment_variable = match name {
        "typikon-antiochian" => "TYPIKON_ANTIOCHIAN_PACK",
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

fn request(date: &str, observances: &[&str]) -> CompileServiceRequest {
    service_request(date, "great_vespers", observances)
}

fn service_request(date: &str, service: &str, observances: &[&str]) -> CompileServiceRequest {
    CompileServiceRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        civil_date: date.to_owned(),
        service: service.to_owned(),
        tone: None,
        phase: None,
        observances: observances
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

#[derive(Debug, Clone, Copy)]
struct MajorFeastCase {
    id: &'static str,
    liturgical_date: &'static str,
    vespers_date: &'static str,
}

const MAJOR_FEASTS_2026: [MajorFeastCase; 12] = [
    MajorFeastCase {
        id: "nativity-theotokos",
        liturgical_date: "2026-09-08",
        vespers_date: "2026-09-07",
    },
    MajorFeastCase {
        id: "elevation-holy-cross",
        liturgical_date: "2026-09-14",
        vespers_date: "2026-09-13",
    },
    MajorFeastCase {
        id: "entrance-theotokos",
        liturgical_date: "2026-11-21",
        vespers_date: "2026-11-20",
    },
    MajorFeastCase {
        id: "nativity-christ",
        liturgical_date: "2026-12-25",
        vespers_date: "2026-12-24",
    },
    MajorFeastCase {
        id: "theophany",
        liturgical_date: "2026-01-06",
        vespers_date: "2026-01-05",
    },
    MajorFeastCase {
        id: "presentation-christ-temple",
        liturgical_date: "2026-02-02",
        vespers_date: "2026-02-01",
    },
    MajorFeastCase {
        id: "annunciation",
        liturgical_date: "2026-03-25",
        vespers_date: "2026-03-24",
    },
    MajorFeastCase {
        id: "palm-sunday",
        liturgical_date: "2026-04-05",
        vespers_date: "2026-04-04",
    },
    MajorFeastCase {
        id: "ascension",
        liturgical_date: "2026-05-21",
        vespers_date: "2026-05-20",
    },
    MajorFeastCase {
        id: "pentecost",
        liturgical_date: "2026-05-31",
        vespers_date: "2026-05-30",
    },
    MajorFeastCase {
        id: "transfiguration",
        liturgical_date: "2026-08-06",
        vespers_date: "2026-08-05",
    },
    MajorFeastCase {
        id: "dormition-theotokos",
        liturgical_date: "2026-08-15",
        vespers_date: "2026-08-14",
    },
];

fn assert_major_feast_services(pack_name: &str, bundle_prefix: &str) {
    let engine = engine(pack_name);
    let reference_prefix = match pack_name {
        "typikon-antiochian" => "https://www.antiochian.org/servicetexts/",
        "typikon-goarch" => "https://digitalchantstand.goarch.org/",
        "typikon-oca" => "https://www.oca.org/",
        _ => unreachable!("test names are fixed"),
    };
    for feast in MAJOR_FEASTS_2026 {
        let plans = engine
            .compile_date(feast.liturgical_date)
            .unwrap_or_else(|error| panic!("{pack_name} {}: {error}", feast.id));
        assert_eq!(
            plans.keys().map(String::as_str).collect::<Vec<_>>(),
            ["divine_liturgy", "matins", "vespers"]
        );
        for (service, date, bundle_service) in [
            ("vespers", feast.vespers_date, "vespers"),
            (
                "matins",
                feast.liturgical_date,
                if pack_name == "typikon-goarch" {
                    "orthros"
                } else {
                    "matins"
                },
            ),
            ("divine_liturgy", feast.liturgical_date, "divine-liturgy"),
        ] {
            let plan = &plans[service];
            let value = serde_json::to_value(&plan).unwrap();
            validate_value(SchemaKind::Plan, "major-feast plan", &value).unwrap();
            assert_eq!(plan.request.civil_date, date);
            assert!(plan.request.observances.is_empty());
            assert_eq!(plan.day.liturgical_date, feast.liturgical_date);
            assert_eq!(plan.observances.len(), 1);
            assert_eq!(plan.observances[0].id, feast.id);
            assert_eq!(plan.sections.len(), 1);
            assert_eq!(plan.sections[0].id, "propers");
            assert_eq!(plan.sections[0].items.len(), 1);
            let material = &plan.sections[0].items[0].material;
            assert_eq!(material["kind"], "feast-propers");
            assert_eq!(material["role"], "complete");
            assert_eq!(material["observance"], feast.id);
            assert_eq!(
                material["bundle"],
                format!(
                    "{bundle_prefix}-{}-{bundle_service}{}",
                    feast.id,
                    if pack_name == "typikon-goarch" {
                        "-2026"
                    } else {
                        ""
                    }
                )
            );
            assert!(
                material["reference"]
                    .as_str()
                    .unwrap()
                    .starts_with(reference_prefix)
            );
            assert_eq!(
                plan.decisions[0].authority[0],
                format!("{bundle_prefix}-major-feast-service-bundles")
            );
            assert!(plan.decisions[0].authority.len() >= 2);
        }
    }
}

fn assert_date_compiles_feast(pack_name: &str, date: &str, feast_id: &str, vespers_date: &str) {
    let plans = engine(pack_name).compile_date(date).unwrap();
    assert_eq!(
        plans.keys().map(String::as_str).collect::<Vec<_>>(),
        ["divine_liturgy", "matins", "vespers"]
    );
    for plan in plans.values() {
        assert!(plan.request.observances.is_empty());
        assert_eq!(plan.day.liturgical_date, date);
        assert_eq!(plan.observances.len(), 1);
        assert_eq!(plan.observances[0].id, feast_id);
    }
    assert_eq!(plans["vespers"].request.civil_date, vespers_date);
    assert_eq!(plans["matins"].request.civil_date, date);
    assert_eq!(plans["divine_liturgy"].request.civil_date, date);
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
        .compile_service(request("2026-07-25", &[]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/goarch-2026-07-25-vespers.json"));
    assert_eq!(plan.day.liturgical_date, "2026-07-26");
    assert_eq!(
        plan.decisions[0].authority,
        ["goarch-dcs-2026-07-25-vespers"]
    );
}

#[test]
fn goarch_dcs_stephen_case_matches_observed_plan() {
    let plan = engine("typikon-goarch")
        .compile_service(request("2026-08-01", &[]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/goarch-2026-08-01-vespers.json"));
    assert_eq!(plan.day.liturgical_date, "2026-08-02");
    assert_eq!(
        plan.decisions[0].authority,
        ["goarch-dcs-2026-08-01-vespers"]
    );
}

#[test]
fn oca_pimen_case_matches_published_seven_plus_three_case() {
    let plan = engine("typikon-oca")
        .compile_service(request("2023-08-26", &[]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/oca-2023-08-26-vespers.json"));
    assert_eq!(plan.day.liturgical_date, "2023-08-27");
    assert_eq!(
        plan.decisions[0].authority,
        ["oca-ordinary-sunday-lord-i-call", "oca-order-2023-08-27"]
    );
}

#[test]
fn oca_archangel_michael_case_matches_published_six_plus_four_case() {
    let plan = engine("typikon-oca")
        .compile_service(request("2026-09-05", &[]))
        .unwrap();

    assert_golden(&plan, include_str!("golden/oca-2026-09-05-vespers.json"));
    assert_eq!(plan.day.liturgical_date, "2026-09-06");
    assert_eq!(
        plan.decisions[0].authority,
        ["oca-ordinary-sunday-lord-i-call", "oca-order-2026-09-06"]
    );
}

#[test]
fn both_traditions_use_the_same_engine_vocabulary() {
    let goarch = engine("typikon-goarch")
        .compile_service(request("2026-07-25", &[]))
        .unwrap();
    let oca = engine("typikon-oca")
        .compile_service(request("2026-09-05", &[]))
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

#[test]
fn oca_major_feasts_compile_all_three_service_bundles() {
    assert_major_feast_services("typikon-oca", "oca");
}

#[test]
fn goarch_major_feasts_compile_all_three_service_bundles() {
    assert_major_feast_services("typikon-goarch", "goarch");
}

#[test]
fn antiochian_major_feasts_compile_all_three_service_bundles() {
    assert_major_feast_services("typikon-antiochian", "antiochian");
}

#[test]
fn pack_and_civil_date_return_the_fixed_feast_service_cycle_without_a_feast_parameter() {
    for pack_name in ["typikon-goarch", "typikon-oca", "typikon-antiochian"] {
        assert_date_compiles_feast(pack_name, "2026-12-25", "nativity-christ", "2026-12-24");
    }
}

#[test]
fn pack_and_civil_date_return_the_movable_feast_service_cycle_without_a_feast_parameter() {
    for pack_name in ["typikon-goarch", "typikon-oca", "typikon-antiochian"] {
        assert_date_compiles_feast(pack_name, "2026-05-31", "pentecost", "2026-05-30");
    }
}

#[test]
fn movable_major_feast_fixtures_follow_the_shared_paschal_cycle() {
    let pascha = NaiveDate::parse_from_str("2026-04-12", "%Y-%m-%d").unwrap();
    let cases = [
        ("palm-sunday", "2026-04-05", -7),
        ("ascension", "2026-05-21", 39),
        ("pentecost", "2026-05-31", 49),
    ];
    for (id, date, expected_offset) in cases {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        assert_eq!((date - pascha).num_days(), expected_offset, "{id}");
    }
}

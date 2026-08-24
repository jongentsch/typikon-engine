#![cfg(feature = "conformance")]

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use typikon_core::Engine;
use typikon_loader::{DirectoryResource, SchemaKind, load_pack, validate_value};
use typikon_schema::{
    CompileServiceRequest, ComponentKind, PlanComponentStatus, PlanStatus, REQUEST_SCHEMA,
};

fn sibling_pack(name: &str) -> PathBuf {
    let variable = match name {
        "typikon-antiochian" => "TYPIKON_ANTIOCHIAN_PACK",
        "typikon-goarch" => "TYPIKON_GOARCH_PACK",
        "typikon-oca" => "TYPIKON_OCA_PACK",
        _ => unreachable!(),
    };
    std::env::var_os(variable).map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../..")
                .join(name)
        },
        PathBuf::from,
    )
}

fn engine(name: &str) -> Engine {
    let resource = DirectoryResource::new(sibling_pack(name)).unwrap();
    Engine::new(load_pack(&resource).unwrap())
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

const MAJOR_FEASTS_2026: [(&str, &str); 12] = [
    ("nativity-theotokos", "2026-09-08"),
    ("elevation-holy-cross", "2026-09-14"),
    ("entrance-theotokos", "2026-11-21"),
    ("nativity-christ", "2026-12-25"),
    ("theophany", "2026-01-06"),
    ("presentation-christ-temple", "2026-02-02"),
    ("annunciation", "2026-03-25"),
    ("palm-sunday", "2026-04-05"),
    ("ascension", "2026-05-21"),
    ("pentecost", "2026-05-31"),
    ("transfiguration", "2026-08-06"),
    ("dormition-theotokos", "2026-08-15"),
];

fn assert_major_feasts(pack_name: &str) {
    let engine = engine(pack_name);
    for (feast, date) in MAJOR_FEASTS_2026 {
        let plans = engine
            .compile_date(date)
            .unwrap_or_else(|error| panic!("{pack_name} {feast}: {error}"));
        assert_eq!(
            plans.keys().map(String::as_str).collect::<Vec<_>>(),
            ["divine_liturgy", "matins", "vespers"]
        );
        for (service_id, plan) in &plans {
            validate_value(
                SchemaKind::Plan,
                "major feast",
                &serde_json::to_value(plan).unwrap(),
            )
            .unwrap();
            assert_eq!(plan.observances[0].id, feast);
            assert_eq!(plan.day.liturgical_date, date);
            assert_eq!(plan.status, PlanStatus::RequiresReview);
            assert!(
                plan.sections.len() > 1,
                "{service_id} must be a full service structure"
            );
            assert!(
                plan.sections
                    .iter()
                    .flat_map(|section| &section.components)
                    .any(|component| {
                        component.kind == ComponentKind::Fixed
                            && component.status == PlanComponentStatus::Resolved
                    })
            );
            assert!(
                plan.sections
                    .iter()
                    .flat_map(|section| &section.components)
                    .any(|component| {
                        component.kind == ComponentKind::Changeable
                            && component.status == PlanComponentStatus::Unresolved
                    })
            );
            let json = serde_json::to_string(plan).unwrap();
            assert!(!json.contains("complete-propers"));
            assert!(!json.contains("service-bundle"));
        }
    }
}

#[test]
fn every_pack_compiles_all_twelve_major_feasts_from_date_alone() {
    for pack in ["typikon-goarch", "typikon-oca", "typikon-antiochian"] {
        assert_major_feasts(pack);
    }
}

#[test]
fn ordinary_saint_material_is_owned_by_the_observance() {
    for (pack, date, observance) in [
        ("typikon-goarch", "2026-07-25", "paraskevi-rome"),
        (
            "typikon-goarch",
            "2026-08-01",
            "stephen-protomartyr-translation",
        ),
        ("typikon-oca", "2023-08-26", "pimen-great"),
        ("typikon-oca", "2026-09-05", "archangel-michael-colossae"),
    ] {
        let plan = engine(pack).compile_service(request(date)).unwrap();
        // The observed Lord-I-Call order is resolved; other universally required
        // Great Vespers components are still surfaced honestly as incomplete.
        assert_eq!(plan.status, PlanStatus::RequiresReview);
        assert_eq!(plan.observances[0].id, observance);
        let lord_i_call = plan
            .sections
            .iter()
            .find(|section| section.id == "lord_i_call")
            .unwrap();
        let stichera = lord_i_call
            .components
            .iter()
            .find(|component| component.id == "stichera")
            .unwrap();
        assert_eq!(stichera.materials.len(), 2);
        assert_eq!(
            stichera.materials[1].observance.as_deref(),
            Some(observance)
        );
        assert!(
            stichera.materials[1].material["title"]
                .as_str()
                .unwrap()
                .contains("stichera")
        );
    }
}

#[test]
fn paschal_offsets_match_the_shared_orthodox_paschalion() {
    let pascha = NaiveDate::parse_from_str("2026-04-12", "%Y-%m-%d").unwrap();
    for (id, date, offset) in [
        ("palm-sunday", "2026-04-05", -7),
        ("ascension", "2026-05-21", 39),
        ("pentecost", "2026-05-31", 49),
    ] {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        assert_eq!((date - pascha).num_days(), offset, "{id}");
    }
}

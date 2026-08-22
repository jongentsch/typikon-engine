//! Typed representations of the language-neutral Typikon definition contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PACK_SCHEMA: &str = "typikon.pack/v0.1";
pub const SERVICE_SCHEMA: &str = "typikon.service/v0.1";
pub const OBSERVANCE_SCHEMA: &str = "typikon.observance/v0.1";
pub const RULE_SCHEMA: &str = "typikon.rule/v0.1";
pub const AUTHORITY_SCHEMA: &str = "typikon.authority/v0.1";
pub const PLAN_SCHEMA: &str = "typikon.plan/v0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackDefinition {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub calendar: CalendarDefinition,
    pub definitions: DefinitionDirectories,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CalendarDefinition {
    pub fixed: String,
    pub paschalion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DefinitionDirectories {
    pub services: String,
    pub observances: String,
    pub rules: String,
    pub authorities: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinition {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub liturgical_day_offset: i32,
    pub sections: Vec<SectionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SectionDefinition {
    pub id: String,
    pub slots: Vec<SlotDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SlotDefinition {
    pub id: String,
    pub cardinality: SlotCardinality,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlotCardinality {
    Many,
    Optional,
    One,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservanceDefinition {
    pub schema: String,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<ObservanceDate>,
    pub rank: String,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservanceDate {
    pub fixed: FixedDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixedDate {
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub schema: String,
    pub id: String,
    pub when: RulePredicate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unless: Option<RulePredicate>,
    pub emit: Vec<EmissionDefinition>,
    #[serde(default)]
    pub authority: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RulePredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<DayPredicate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observance: Option<ObservancePredicate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DayPredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekday: Option<OneOrMany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<OneOrMany>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservancePredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<OneOrMany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<OneOrMany>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    #[must_use]
    pub fn contains(&self, candidate: &str) -> bool {
        match self {
            Self::One(value) => value == candidate,
            Self::Many(values) => values.iter().any(|value| value == candidate),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmissionDefinition {
    pub section: String,
    pub slot: String,
    pub material: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDefinition {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub kind: AuthorityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default)]
    pub locator: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<AuthorityReference>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    Authoritative,
    ObservedBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityReference {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompileServiceRequest {
    pub civil_date: String,
    pub service: String,
    pub tone: String,
    #[serde(default = "ordinary_phase")]
    pub phase: String,
    #[serde(default)]
    pub observances: Vec<String>,
}

fn ordinary_phase() -> String {
    "ordinary".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub engine_version: String,
    pub status: PlanStatus,
    pub pack: PlanPack,
    pub request: CompileServiceRequest,
    pub day: LiturgicalDay,
    pub observances: Vec<PlanObservance>,
    pub sections: Vec<PlanSection>,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Complete,
    RequiresReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanPack {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LiturgicalDay {
    pub liturgical_date: String,
    pub weekday: String,
    pub tone: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanObservance {
    pub id: String,
    pub name: String,
    pub rank: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanSection {
    pub id: String,
    pub items: Vec<PlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    pub slot: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    pub material: BTreeMap<String, Value>,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub rule: String,
    pub authority: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observance: Option<String>,
}

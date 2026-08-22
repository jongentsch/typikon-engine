//! Pure, deterministic rule matching and semantic plan assembly.

pub mod calendar;

use std::collections::{BTreeMap, BTreeSet};

use calendar::{
    CalendarDate, CalendarError, PhaseComputation, ToneComputation, liturgical_phase,
    octoechos_tone, orthodox_pascha, project_fixed_date,
};
use chrono::{Datelike, Duration, NaiveDate};
use serde_json::Value;
use thiserror::Error;
use typikon_loader::{LoadedPack, SchemaKind, validate_value};
use typikon_schema::{
    CalendarDefinition, CompileServiceRequest, DayPredicate, Decision, LiturgicalDay,
    ObservanceDefinition, ObservancePredicate, PLAN_SCHEMA, Plan, PlanDerivation, PlanItem,
    PlanObservance, PlanPack, PlanSection, PlanStatus, REQUEST_SCHEMA, RuleDefinition,
    RulePredicate, ServiceDefinition, SlotCardinality,
};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct Engine {
    pack: LoadedPack,
}

impl Engine {
    #[must_use]
    pub fn new(pack: LoadedPack) -> Self {
        Self { pack }
    }

    #[must_use]
    pub const fn pack(&self) -> &LoadedPack {
        &self.pack
    }

    /// Compiles one service plan from caller-supplied context and this engine's pack.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid request context, absent matches, unresolved
    /// variables, or ambiguous/missing exclusive slots.
    pub fn compile_service(&self, request: CompileServiceRequest) -> Result<Plan, EngineError> {
        if request.schema != REQUEST_SCHEMA {
            return Err(EngineError::UnsupportedRequestSchema(request.schema));
        }
        if let Some(tone) = &request.tone {
            validate_context_identifier("tone", tone)?;
        }
        if let Some(phase) = &request.phase {
            validate_context_identifier("phase", phase)?;
        }
        let service = self
            .pack
            .services
            .get(&request.service)
            .ok_or_else(|| EngineError::UnknownService(request.service.clone()))?;
        let (fixed_date, day, derivations) = build_day(
            &request,
            service.value.liturgical_day_offset,
            &self.pack.pack.value.calendar,
        )?;

        let automatic_observances = request.observances.is_empty();
        let observance_ids = self.select_observances(&request, &fixed_date)?;
        let observances = observance_ids
            .iter()
            .map(|id| {
                self.pack
                    .observances
                    .get(id)
                    .map(|sourced| &sourced.value)
                    .ok_or_else(|| EngineError::UnknownObservance(id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (sections, decisions) =
            self.evaluate_rules(&request.service, &day, &observances, &service.value)?;

        if decisions.is_empty() {
            return Err(EngineError::NoMatchingRules {
                service: request.service.clone(),
                date: day.liturgical_date.clone(),
            });
        }
        validate_required_slots(&service.value, &sections)?;

        Ok(Plan {
            schema: PLAN_SCHEMA.to_owned(),
            engine_version: ENGINE_VERSION.to_owned(),
            status: PlanStatus::Complete,
            pack: PlanPack {
                id: self.pack.pack.value.id.clone(),
                version: self.pack.pack.value.version.clone(),
            },
            request,
            day,
            derivations,
            observances: observances
                .into_iter()
                .map(|observance| PlanObservance {
                    id: observance.id.clone(),
                    name: observance.name.clone(),
                    rank: observance.rank.clone(),
                    selection_derivation: automatic_observances
                        .then(|| "derivation-0002".to_owned()),
                })
                .collect(),
            sections,
            decisions,
        })
    }

    /// Validates and compiles a versioned UTF-8 JSON request into deterministic
    /// UTF-8 JSON conforming to the plan contract.
    ///
    /// # Errors
    ///
    /// Returns an interoperability error for malformed JSON, request or plan
    /// schema violations, typed deserialization errors, or compilation errors.
    pub fn compile_service_json(&self, request_json: &str) -> Result<String, InteropError> {
        let request_value: Value = serde_json::from_str(request_json)
            .map_err(|error| InteropError::MalformedRequest(error.to_string()))?;
        validate_value(SchemaKind::Request, "compile request", &request_value)
            .map_err(|error| InteropError::InvalidRequest(error.to_string()))?;
        let request = serde_json::from_value(request_value)
            .map_err(|error| InteropError::InvalidRequest(error.to_string()))?;
        let plan = self.compile_service(request)?;
        let plan_value = serde_json::to_value(plan)
            .map_err(|error| InteropError::PlanSerialization(error.to_string()))?;
        validate_value(SchemaKind::Plan, "compiled plan", &plan_value)
            .map_err(|error| InteropError::InvalidPlan(error.to_string()))?;
        serde_json::to_string(&plan_value)
            .map_err(|error| InteropError::PlanSerialization(error.to_string()))
    }

    fn evaluate_rules(
        &self,
        service_id: &str,
        day: &LiturgicalDay,
        observances: &[&ObservanceDefinition],
        service: &ServiceDefinition,
    ) -> Result<(Vec<PlanSection>, Vec<Decision>), EngineError> {
        let mut sections = service
            .sections
            .iter()
            .map(|section| PlanSection {
                id: section.id.clone(),
                items: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut decisions = Vec::new();

        for sourced_rule in self.pack.rules.values() {
            let rule = &sourced_rule.value;
            let candidates = if rule.when.observance.is_some() {
                observances.iter().copied().map(Some).collect::<Vec<_>>()
            } else {
                vec![None]
            };
            for candidate in candidates {
                let matches =
                    predicate_matches(&rule.when, service_id, day, candidate, observances);
                let excluded = rule.unless.as_ref().is_some_and(|unless| {
                    predicate_matches(unless, service_id, day, None, observances)
                });
                if matches && !excluded {
                    emit_rule_match(rule, service, day, candidate, &mut sections, &mut decisions)?;
                }
            }
        }
        Ok((sections, decisions))
    }

    fn select_observances(
        &self,
        request: &CompileServiceRequest,
        fixed_date: &CalendarDate,
    ) -> Result<Vec<String>, EngineError> {
        if request.observances.is_empty() {
            return Ok(self
                .pack
                .observances
                .iter()
                .filter(|(_, sourced)| {
                    sourced.value.date.as_ref().is_some_and(|date| {
                        u32::from(date.fixed.month) == fixed_date.month
                            && u32::from(date.fixed.day) == fixed_date.day
                    })
                })
                .map(|(id, _)| id.clone())
                .collect());
        }

        let mut selected = BTreeSet::new();
        for id in &request.observances {
            if !self.pack.observances.contains_key(id) {
                return Err(EngineError::UnknownObservance(id.clone()));
            }
            if !selected.insert(id.clone()) {
                return Err(EngineError::DuplicateObservance(id.clone()));
            }
        }
        Ok(selected.into_iter().collect())
    }
}

fn emit_rule_match(
    rule: &RuleDefinition,
    service: &ServiceDefinition,
    day: &LiturgicalDay,
    candidate: Option<&ObservanceDefinition>,
    sections: &mut [PlanSection],
    decisions: &mut Vec<Decision>,
) -> Result<(), EngineError> {
    let decision_id = format!("decision-{:04}", decisions.len() + 1);
    for emission in &rule.emit {
        let section_index = service
            .sections
            .iter()
            .position(|section| section.id == emission.section)
            .ok_or_else(|| EngineError::InvalidPackReference {
                rule: rule.id.clone(),
                reference: format!("section '{}'", emission.section),
            })?;
        let slot = service.sections[section_index]
            .slots
            .iter()
            .find(|slot| slot.id == emission.slot)
            .ok_or_else(|| EngineError::InvalidPackReference {
                rule: rule.id.clone(),
                reference: format!("slot '{}:{}'", emission.section, emission.slot),
            })?;

        if slot.cardinality != SlotCardinality::Many {
            check_slot_ambiguity(
                rule,
                emission.section.as_str(),
                emission.slot.as_str(),
                &sections[section_index],
                decisions,
            )?;
        }

        let material =
            resolve_material(&emission.material, day, candidate).map_err(|variable| {
                EngineError::UnknownVariable {
                    rule: rule.id.clone(),
                    variable,
                }
            })?;
        sections[section_index].items.push(PlanItem {
            slot: emission.slot.clone(),
            count: emission.count,
            material,
            decision: decision_id.clone(),
        });
    }
    let mut seen_authorities = BTreeSet::new();
    let authority = rule
        .authority
        .iter()
        .chain(candidate.into_iter().flat_map(|value| &value.authority))
        .filter(|value| seen_authorities.insert(value.as_str()))
        .cloned()
        .collect();
    decisions.push(Decision {
        id: decision_id,
        rule: rule.id.clone(),
        authority,
        observance: candidate.map(|observance| observance.id.clone()),
    });
    Ok(())
}

fn check_slot_ambiguity(
    rule: &RuleDefinition,
    section_id: &str,
    slot_id: &str,
    section: &PlanSection,
    decisions: &[Decision],
) -> Result<(), EngineError> {
    let Some(existing) = section.items.iter().find(|item| item.slot == slot_id) else {
        return Ok(());
    };
    let first_rule = decisions
        .iter()
        .find(|decision| decision.id == existing.decision)
        .map_or("unknown", |decision| decision.rule.as_str());
    Err(EngineError::AmbiguousSlot {
        section: section_id.to_owned(),
        slot: slot_id.to_owned(),
        first_rule: first_rule.to_owned(),
        second_rule: rule.id.clone(),
    })
}

fn resolve_material(
    material: &BTreeMap<String, Value>,
    day: &LiturgicalDay,
    candidate: Option<&ObservanceDefinition>,
) -> Result<BTreeMap<String, Value>, String> {
    material
        .iter()
        .map(|(key, value)| {
            resolve_value(value, day, candidate).map(|resolved| (key.clone(), resolved))
        })
        .collect()
}

fn build_day(
    request: &CompileServiceRequest,
    liturgical_day_offset: i32,
    calendar: &CalendarDefinition,
) -> Result<(CalendarDate, LiturgicalDay, Vec<PlanDerivation>), EngineError> {
    if !has_iso_date_shape(&request.civil_date) {
        return Err(EngineError::InvalidCivilDate {
            value: request.civil_date.clone(),
            message: "expected YYYY-MM-DD".to_owned(),
        });
    }
    let civil_date =
        NaiveDate::parse_from_str(&request.civil_date, "%Y-%m-%d").map_err(|error| {
            EngineError::InvalidCivilDate {
                value: request.civil_date.clone(),
                message: error.to_string(),
            }
        })?;
    let date = civil_date
        .checked_add_signed(Duration::days(i64::from(liturgical_day_offset)))
        .ok_or(EngineError::DateOverflow)?;
    let fixed_date = project_fixed_date(date, calendar.fixed)?;
    let pascha = orthodox_pascha(date.year())?;
    let tone = octoechos_tone(date, &calendar.tone_cycle.tones)?;
    let phase = liturgical_phase(date)?;
    if let Some(requested) = &request.tone {
        let calculated = tone.as_ref().map(|value| value.tone.as_str());
        if Some(requested.as_str()) != calculated {
            return Err(EngineError::ToneMismatch {
                requested: requested.clone(),
                calculated: calculated.map(str::to_owned),
            });
        }
    }
    if let Some(requested) = &request.phase
        && requested != phase.phase
    {
        return Err(EngineError::PhaseMismatch {
            requested: requested.clone(),
            calculated: phase.phase.to_owned(),
        });
    }
    let day = LiturgicalDay {
        liturgical_date: date.format("%Y-%m-%d").to_string(),
        fixed_date: fixed_date.to_string(),
        fixed_calendar: calendar.fixed,
        pascha: pascha.to_string(),
        weekday: weekday_name(date).to_owned(),
        tone: tone.as_ref().map(|value| value.tone.clone()),
        phase: phase.phase.to_owned(),
    };
    let derivations = build_day_derivations(
        request,
        liturgical_day_offset,
        calendar,
        &day,
        tone.as_ref(),
        &phase,
        date,
    );
    Ok((fixed_date, day, derivations))
}

fn build_day_derivations(
    request: &CompileServiceRequest,
    liturgical_day_offset: i32,
    calendar: &CalendarDefinition,
    day: &LiturgicalDay,
    tone: Option<&ToneComputation>,
    phase: &PhaseComputation,
    date: NaiveDate,
) -> Vec<PlanDerivation> {
    let derivations = vec![
        PlanDerivation {
            id: "derivation-0001".to_owned(),
            component: "liturgical_date".to_owned(),
            method: "service_day_offset".to_owned(),
            inputs: BTreeMap::from([
                (
                    "civil_date".to_owned(),
                    Value::String(request.civil_date.clone()),
                ),
                (
                    "offset_days".to_owned(),
                    Value::Number(liturgical_day_offset.into()),
                ),
            ]),
            output: Value::String(day.liturgical_date.clone()),
        },
        PlanDerivation {
            id: "derivation-0002".to_owned(),
            component: "fixed_date".to_owned(),
            method: "calendar_projection".to_owned(),
            inputs: BTreeMap::from([
                (
                    "calendar".to_owned(),
                    serde_json::to_value(calendar.fixed).expect("calendar enum always serializes"),
                ),
                (
                    "liturgical_date".to_owned(),
                    Value::String(day.liturgical_date.clone()),
                ),
            ]),
            output: Value::String(day.fixed_date.clone()),
        },
        PlanDerivation {
            id: "derivation-0003".to_owned(),
            component: "pascha".to_owned(),
            method: "orthodox_julian_computus".to_owned(),
            inputs: BTreeMap::from([("year".to_owned(), Value::Number(date.year().into()))]),
            output: Value::String(day.pascha.clone()),
        },
        PlanDerivation {
            id: "derivation-0004".to_owned(),
            component: "weekday".to_owned(),
            method: "proleptic_gregorian_weekday".to_owned(),
            inputs: BTreeMap::from([(
                "liturgical_date".to_owned(),
                Value::String(day.liturgical_date.clone()),
            )]),
            output: Value::String(day.weekday.clone()),
        },
        PlanDerivation {
            id: "derivation-0005".to_owned(),
            component: "phase".to_owned(),
            method: "paschal_cycle_window".to_owned(),
            inputs: BTreeMap::from([
                ("pascha".to_owned(), Value::String(phase.pascha.to_string())),
                (
                    "pentecostarion_end".to_owned(),
                    Value::String(phase.pentecostarion_end.to_string()),
                ),
                (
                    "triodion_start".to_owned(),
                    Value::String(phase.triodion_start.to_string()),
                ),
            ]),
            output: Value::String(day.phase.clone()),
        },
        tone_derivation(tone, phase),
    ];
    derivations
}

fn tone_derivation(tone: Option<&ToneComputation>, phase: &PhaseComputation) -> PlanDerivation {
    tone.map_or_else(
        || PlanDerivation {
            id: "derivation-0006".to_owned(),
            component: "tone".to_owned(),
            method: "octoechos_suspended_bright_week".to_owned(),
            inputs: BTreeMap::from([
                ("pascha".to_owned(), Value::String(phase.pascha.to_string())),
                (
                    "resumes".to_owned(),
                    Value::String(
                        phase
                            .pascha
                            .checked_add_signed(Duration::days(7))
                            .expect("validated calendar arithmetic")
                            .to_string(),
                    ),
                ),
            ]),
            output: Value::Null,
        },
        |tone| PlanDerivation {
            id: "derivation-0006".to_owned(),
            component: "tone".to_owned(),
            method: "octoechos_sunday_after_pascha".to_owned(),
            inputs: BTreeMap::from([
                ("anchor".to_owned(), Value::String(tone.anchor.to_string())),
                ("ordinal".to_owned(), Value::Number(tone.ordinal.into())),
                (
                    "weeks_from_anchor".to_owned(),
                    Value::Number(tone.weeks_from_anchor.into()),
                ),
            ]),
            output: Value::String(tone.tone.clone()),
        },
    )
}

fn validate_context_identifier(field: &'static str, value: &str) -> Result<(), EngineError> {
    let valid = value
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '-')
        })
        && value
            .chars()
            .last()
            .is_some_and(|last| last.is_ascii_lowercase() || last.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(EngineError::InvalidContextValue {
            field,
            value: value.to_owned(),
        })
    }
}

fn has_iso_date_shape(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn validate_required_slots(
    service: &ServiceDefinition,
    sections: &[PlanSection],
) -> Result<(), EngineError> {
    for (section_definition, section) in service.sections.iter().zip(sections) {
        for slot in &section_definition.slots {
            if slot.cardinality == SlotCardinality::One
                && !section.items.iter().any(|item| item.slot == slot.id)
            {
                return Err(EngineError::MissingRequiredSlot {
                    section: section.id.clone(),
                    slot: slot.id.clone(),
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("unsupported request schema '{0}'")]
    UnsupportedRequestSchema(String),
    #[error("unknown service '{0}'")]
    UnknownService(String),
    #[error("unknown observance '{0}'")]
    UnknownObservance(String),
    #[error("observance '{0}' was selected more than once")]
    DuplicateObservance(String),
    #[error("invalid civil date '{value}': {message}")]
    InvalidCivilDate { value: String, message: String },
    #[error("invalid {field} context value '{value}'")]
    InvalidContextValue { field: &'static str, value: String },
    #[error("liturgical date overflowed the supported date range")]
    DateOverflow,
    #[error("requested tone '{requested}' does not match calculated tone {calculated:?}")]
    ToneMismatch {
        requested: String,
        calculated: Option<String>,
    },
    #[error("requested phase '{requested}' does not match calculated phase '{calculated}'")]
    PhaseMismatch {
        requested: String,
        calculated: String,
    },
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    #[error("no rules matched service '{service}' for liturgical date {date}")]
    NoMatchingRules { service: String, date: String },
    #[error("rule '{rule}' uses unknown variable '{variable}'")]
    UnknownVariable { rule: String, variable: String },
    #[error("rule '{rule}' has invalid validated-pack reference: {reference}")]
    InvalidPackReference { rule: String, reference: String },
    #[error(
        "conflicting emissions for exclusive slot {section}:{slot} from rules '{first_rule}' and '{second_rule}'"
    )]
    AmbiguousSlot {
        section: String,
        slot: String,
        first_rule: String,
        second_rule: String,
    },
    #[error("required slot {section}:{slot} has no emitted item")]
    MissingRequiredSlot { section: String, slot: String },
}

#[derive(Debug, Error)]
pub enum InteropError {
    #[error("malformed request JSON: {0}")]
    MalformedRequest(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Compilation(#[from] EngineError),
    #[error("compiled plan failed its schema contract: {0}")]
    InvalidPlan(String),
    #[error("cannot serialize compiled plan: {0}")]
    PlanSerialization(String),
}

fn predicate_matches(
    predicate: &RulePredicate,
    service: &str,
    day: &LiturgicalDay,
    candidate: Option<&ObservanceDefinition>,
    observances: &[&ObservanceDefinition],
) -> bool {
    if predicate
        .service
        .as_ref()
        .is_some_and(|expected| expected != service)
    {
        return false;
    }
    if predicate
        .day
        .as_ref()
        .is_some_and(|expected| !day_matches(expected, day))
    {
        return false;
    }
    predicate.observance.as_ref().is_none_or(|expected| {
        candidate.map_or_else(
            || {
                observances
                    .iter()
                    .any(|observance| observance_matches(expected, observance))
            },
            |observance| observance_matches(expected, observance),
        )
    })
}

fn day_matches(predicate: &DayPredicate, day: &LiturgicalDay) -> bool {
    predicate
        .weekday
        .as_ref()
        .is_none_or(|expected| expected.contains(&day.weekday))
        && predicate
            .phase
            .as_ref()
            .is_none_or(|expected| expected.contains(&day.phase))
}

fn observance_matches(predicate: &ObservancePredicate, observance: &ObservanceDefinition) -> bool {
    predicate
        .id
        .as_ref()
        .is_none_or(|expected| expected.contains(&observance.id))
        && predicate
            .rank
            .as_ref()
            .is_none_or(|expected| expected.contains(&observance.rank))
        && predicate
            .properties
            .iter()
            .all(|(key, expected)| observance.properties.get(key) == Some(expected))
}

fn resolve_value(
    value: &Value,
    day: &LiturgicalDay,
    observance: Option<&ObservanceDefinition>,
) -> Result<Value, String> {
    let Some(variable) = value.as_str().filter(|value| value.starts_with('$')) else {
        return Ok(value.clone());
    };
    match variable {
        "$day.liturgical_date" => Ok(Value::String(day.liturgical_date.clone())),
        "$day.weekday" => Ok(Value::String(day.weekday.clone())),
        "$day.tone" => day
            .tone
            .as_ref()
            .map(|tone| Value::String(tone.clone()))
            .ok_or_else(|| variable.to_owned()),
        "$day.phase" => Ok(Value::String(day.phase.clone())),
        "$observance.id" => observance
            .map(|value| Value::String(value.id.clone()))
            .ok_or_else(|| variable.to_owned()),
        "$observance.rank" => observance
            .map(|value| Value::String(value.rank.clone()))
            .ok_or_else(|| variable.to_owned()),
        _ => variable
            .strip_prefix("$observance.properties.")
            .and_then(|property| {
                observance.and_then(|value| value.properties.get(property).cloned())
            })
            .ok_or_else(|| variable.to_owned()),
    }
}

fn weekday_name(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "monday",
        chrono::Weekday::Tue => "tuesday",
        chrono::Weekday::Wed => "wednesday",
        chrono::Weekday::Thu => "thursday",
        chrono::Weekday::Fri => "friday",
        chrono::Weekday::Sat => "saturday",
        chrono::Weekday::Sun => "sunday",
    }
}

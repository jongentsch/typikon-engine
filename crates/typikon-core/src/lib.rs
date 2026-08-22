//! Pure, deterministic rule matching and semantic plan assembly.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde_json::Value;
use thiserror::Error;
use typikon_loader::LoadedPack;
use typikon_schema::{
    CompileServiceRequest, DayPredicate, Decision, LiturgicalDay, ObservanceDefinition,
    ObservancePredicate, PLAN_SCHEMA, Plan, PlanItem, PlanObservance, PlanPack, PlanSection,
    PlanStatus, RuleDefinition, RulePredicate, ServiceDefinition, SlotCardinality,
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
    pub fn compile_service(&self, mut request: CompileServiceRequest) -> Result<Plan, EngineError> {
        validate_context_identifier("tone", &request.tone)?;
        validate_context_identifier("phase", &request.phase)?;
        let service = self
            .pack
            .services
            .get(&request.service)
            .ok_or_else(|| EngineError::UnknownService(request.service.clone()))?;
        let (liturgical_date, day) = build_day(&request, service.value.liturgical_day_offset)?;

        let observance_ids = self.select_observances(&request, liturgical_date)?;
        request.observances.clone_from(&observance_ids);
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
            observances: observances
                .into_iter()
                .map(|observance| PlanObservance {
                    id: observance.id.clone(),
                    name: observance.name.clone(),
                    rank: observance.rank.clone(),
                })
                .collect(),
            sections,
            decisions,
        })
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
        liturgical_date: NaiveDate,
    ) -> Result<Vec<String>, EngineError> {
        if request.observances.is_empty() {
            return Ok(self
                .pack
                .observances
                .iter()
                .filter(|(_, sourced)| {
                    sourced.value.date.as_ref().is_some_and(|date| {
                        u32::from(date.fixed.month) == liturgical_date.month()
                            && u32::from(date.fixed.day) == liturgical_date.day()
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
) -> Result<(NaiveDate, LiturgicalDay), EngineError> {
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
    let day = LiturgicalDay {
        liturgical_date: date.format("%Y-%m-%d").to_string(),
        weekday: weekday_name(date).to_owned(),
        tone: request.tone.clone(),
        phase: request.phase.clone(),
    };
    Ok((date, day))
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
        "$day.tone" => Ok(Value::String(day.tone.clone())),
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

//! Runtime resource loading, schema validation, and reference validation.

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::path::{Component, Path};

use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;
use typikon_schema::{
    AuthorityCategory, AuthorityDefinition, LiturgicalResourceDefinition, ObservanceDate,
    ObservanceDefinition, PackDefinition, RuleDefinition, ServiceDefinition,
};

const PACK_JSON_SCHEMA: &str = include_str!("../../../schemas/pack.schema.json");
const SERVICE_JSON_SCHEMA: &str = include_str!("../../../schemas/service.schema.json");
const OBSERVANCE_JSON_SCHEMA: &str = include_str!("../../../schemas/observance.schema.json");
const RULE_JSON_SCHEMA: &str = include_str!("../../../schemas/rule.schema.json");
const AUTHORITY_JSON_SCHEMA: &str = include_str!("../../../schemas/authority.schema.json");
const RESOURCE_JSON_SCHEMA: &str = include_str!("../../../schemas/resource.schema.json");
const FFI_RESPONSE_JSON_SCHEMA: &str = include_str!("../../../schemas/ffi-response.schema.json");
const REQUEST_JSON_SCHEMA: &str = include_str!("../../../schemas/request.schema.json");
const RESOURCE_BUNDLE_JSON_SCHEMA: &str =
    include_str!("../../../schemas/resource-bundle.schema.json");
const PLAN_JSON_SCHEMA: &str = include_str!("../../../schemas/plan.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Pack,
    Service,
    Observance,
    Rule,
    Authority,
    Resource,
    FfiResponse,
    Request,
    ResourceBundle,
    Plan,
}

impl SchemaKind {
    const fn document_schema(self) -> &'static str {
        match self {
            Self::Pack => PACK_JSON_SCHEMA,
            Self::Service => SERVICE_JSON_SCHEMA,
            Self::Observance => OBSERVANCE_JSON_SCHEMA,
            Self::Rule => RULE_JSON_SCHEMA,
            Self::Authority => AUTHORITY_JSON_SCHEMA,
            Self::Resource => RESOURCE_JSON_SCHEMA,
            Self::FfiResponse => FFI_RESPONSE_JSON_SCHEMA,
            Self::Request => REQUEST_JSON_SCHEMA,
            Self::ResourceBundle => RESOURCE_BUNDLE_JSON_SCHEMA,
            Self::Plan => PLAN_JSON_SCHEMA,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Pack => "pack",
            Self::Service => "service",
            Self::Observance => "observance",
            Self::Rule => "rule",
            Self::Authority => "authority",
            Self::Resource => "liturgical resource",
            Self::FfiResponse => "FFI response",
            Self::Request => "request",
            Self::ResourceBundle => "resource bundle",
            Self::Plan => "plan",
        }
    }
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("resource path must be relative and may not contain parent traversal: {0}")]
    UnsafePath(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("cannot access resource {path}: {message}")]
    Io { path: String, message: String },
}

pub trait TraditionResource {
    /// Read a file identified relative to this resource's root.
    ///
    /// # Errors
    ///
    /// Returns a resource error when the path is unsafe, missing, or unreadable.
    fn read(&self, path: &str) -> Result<Vec<u8>, ResourceError>;

    /// List YAML files recursively beneath a relative directory.
    ///
    /// # Errors
    ///
    /// Returns a resource error when the directory is unsafe, missing, or unreadable.
    fn list_yaml(&self, directory: &str) -> Result<Vec<String>, ResourceError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryResource {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemoryResource {
    #[must_use]
    pub fn new(files: BTreeMap<String, Vec<u8>>) -> Self {
        Self { files }
    }

    #[must_use]
    pub fn from_text(files: impl IntoIterator<Item = (String, String)>) -> Self {
        Self::new(
            files
                .into_iter()
                .map(|(path, contents)| (normalize_resource_path(&path), contents.into_bytes()))
                .collect(),
        )
    }
}

impl TraditionResource for MemoryResource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ResourceError> {
        ensure_safe_relative(path)?;
        let normalized = normalize_resource_path(path);
        self.files
            .get(&normalized)
            .cloned()
            .ok_or(ResourceError::NotFound(normalized))
    }

    fn list_yaml(&self, directory: &str) -> Result<Vec<String>, ResourceError> {
        ensure_safe_relative(directory)?;
        let prefix = format!(
            "{}/",
            normalize_resource_path(directory).trim_end_matches('/')
        );
        let mut paths = self
            .files
            .keys()
            .filter(|path| path.starts_with(&prefix) && has_yaml_extension(Path::new(path)))
            .cloned()
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }
}

#[cfg(feature = "filesystem")]
#[derive(Debug, Clone)]
pub struct DirectoryResource {
    root: std::path::PathBuf,
}

#[cfg(feature = "filesystem")]
impl DirectoryResource {
    /// Confines filesystem access to an existing pack directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the root is missing, inaccessible, or not a directory.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ResourceError> {
        let display = root.as_ref().display().to_string();
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|error| ResourceError::Io {
                path: display,
                message: error.to_string(),
            })?;
        if !root.is_dir() {
            return Err(ResourceError::Io {
                path: root.display().to_string(),
                message: "pack root is not a directory".to_owned(),
            });
        }
        Ok(Self { root })
    }

    fn resolve_existing(&self, relative: &str) -> Result<std::path::PathBuf, ResourceError> {
        ensure_safe_relative(relative)?;
        let candidate = self.root.join(relative);
        let resolved = candidate
            .canonicalize()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => ResourceError::NotFound(relative.to_owned()),
                _ => ResourceError::Io {
                    path: relative.to_owned(),
                    message: error.to_string(),
                },
            })?;
        if !resolved.starts_with(&self.root) {
            return Err(ResourceError::UnsafePath(relative.to_owned()));
        }
        Ok(resolved)
    }
}

#[cfg(feature = "filesystem")]
impl TraditionResource for DirectoryResource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ResourceError> {
        let resolved = self.resolve_existing(path)?;
        std::fs::read(resolved).map_err(|error| ResourceError::Io {
            path: path.to_owned(),
            message: error.to_string(),
        })
    }

    fn list_yaml(&self, directory: &str) -> Result<Vec<String>, ResourceError> {
        let resolved = self.resolve_existing(directory)?;
        if !resolved.is_dir() {
            return Err(ResourceError::Io {
                path: directory.to_owned(),
                message: "definition path is not a directory".to_owned(),
            });
        }

        let mut pending = vec![resolved];
        let mut paths = Vec::new();
        while let Some(current) = pending.pop() {
            let entries = std::fs::read_dir(&current).map_err(|error| ResourceError::Io {
                path: current.display().to_string(),
                message: error.to_string(),
            })?;
            for entry in entries {
                let entry = entry.map_err(|error| ResourceError::Io {
                    path: current.display().to_string(),
                    message: error.to_string(),
                })?;
                let file_type = entry.file_type().map_err(|error| ResourceError::Io {
                    path: entry.path().display().to_string(),
                    message: error.to_string(),
                })?;
                if file_type.is_dir() {
                    pending.push(entry.path());
                } else if file_type.is_file() && has_yaml_extension(&entry.path()) {
                    let entry_path = entry.path();
                    let relative =
                        entry_path
                            .strip_prefix(&self.root)
                            .map_err(|error| ResourceError::Io {
                                path: entry_path.display().to_string(),
                                message: error.to_string(),
                            })?;
                    paths.push(normalize_resource_path(&relative.to_string_lossy()));
                }
            }
        }
        paths.sort();
        Ok(paths)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sourced<T> {
    pub source: String,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPack {
    pub pack: Sourced<PackDefinition>,
    pub services: BTreeMap<String, Sourced<ServiceDefinition>>,
    pub observances: BTreeMap<String, Sourced<ObservanceDefinition>>,
    pub resources: BTreeMap<String, Sourced<LiturgicalResourceDefinition>>,
    pub rules: BTreeMap<String, Sourced<RuleDefinition>>,
    pub authorities: BTreeMap<String, Sourced<AuthorityDefinition>>,
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("{path}: {source}")]
    Resource {
        path: String,
        #[source]
        source: ResourceError,
    },
    #[error("{path}: malformed YAML: {message}")]
    MalformedYaml { path: String, message: String },
    #[error("{path}: invalid {kind} definition: {message}")]
    Schema {
        path: String,
        kind: &'static str,
        message: String,
    },
    #[error("{path}: cannot deserialize validated {kind}: {message}")]
    Deserialize {
        path: String,
        kind: &'static str,
        message: String,
    },
    #[error("duplicate {kind} id '{id}' in {first} and {second}")]
    DuplicateId {
        kind: &'static str,
        id: String,
        first: String,
        second: String,
    },
    #[error("{path}: duplicate {kind} id '{id}' in {owner}")]
    DuplicateNestedId {
        path: String,
        kind: &'static str,
        id: String,
        owner: String,
    },
    #[error("{path}: {owner} references unknown {kind} '{id}'")]
    UnknownReference {
        path: String,
        owner: String,
        kind: &'static str,
        id: String,
    },
    #[error("{path}: observance '{id}' has impossible fixed date {month:02}-{day:02}")]
    InvalidFixedDate {
        path: String,
        id: String,
        month: u8,
        day: u8,
    },
    #[error("pack has no {0} definitions")]
    EmptyDefinitionSet(&'static str),
}

/// Loads, schema-validates, deserializes, and reference-checks a tradition pack.
///
/// # Errors
///
/// Returns a source-aware error for inaccessible resources, malformed YAML,
/// schema violations, duplicate IDs, or invalid references.
pub fn load_pack(resource: &impl TraditionResource) -> Result<LoadedPack, LoaderError> {
    let pack = load_document::<PackDefinition>(resource, "pack.yaml", SchemaKind::Pack)?;
    let services = load_collection(
        resource,
        &pack.value.definitions.services,
        SchemaKind::Service,
        |value: &ServiceDefinition| &value.id,
    )?;
    let observances = load_collection(
        resource,
        &pack.value.definitions.observances,
        SchemaKind::Observance,
        |value: &ObservanceDefinition| &value.id,
    )?;
    let resources = load_collection(
        resource,
        &pack.value.definitions.resources,
        SchemaKind::Resource,
        |value: &LiturgicalResourceDefinition| &value.id,
    )?;
    let rules = load_collection(
        resource,
        &pack.value.definitions.rules,
        SchemaKind::Rule,
        |value: &RuleDefinition| &value.id,
    )?;
    let authorities = load_collection(
        resource,
        &pack.value.definitions.authorities,
        SchemaKind::Authority,
        |value: &AuthorityDefinition| &value.id,
    )?;

    if services.is_empty() {
        return Err(LoaderError::EmptyDefinitionSet("service"));
    }
    if rules.is_empty() {
        return Err(LoaderError::EmptyDefinitionSet("rule"));
    }

    let loaded = LoadedPack {
        pack,
        services,
        observances,
        resources,
        rules,
        authorities,
    };
    validate_references(&loaded)?;
    Ok(loaded)
}

/// Validates a JSON-compatible value against one embedded contract schema.
///
/// # Errors
///
/// Returns a schema error containing all validation failures.
pub fn validate_value(
    kind: SchemaKind,
    path: impl Into<String>,
    value: &Value,
) -> Result<(), LoaderError> {
    let path = path.into();
    let schema =
        serde_json::from_str(kind.document_schema()).map_err(|error| LoaderError::Schema {
            path: path.clone(),
            kind: kind.label(),
            message: format!("embedded engine schema is invalid: {error}"),
        })?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| LoaderError::Schema {
        path: path.clone(),
        kind: kind.label(),
        message: format!("embedded engine schema cannot be compiled: {error}"),
    })?;
    let errors = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(LoaderError::Schema {
            path,
            kind: kind.label(),
            message: errors.join("; "),
        })
    }
}

fn load_collection<T>(
    resource: &impl TraditionResource,
    directory: &str,
    kind: SchemaKind,
    id: impl Fn(&T) -> &String,
) -> Result<BTreeMap<String, Sourced<T>>, LoaderError>
where
    T: DeserializeOwned,
{
    let paths = resource
        .list_yaml(directory)
        .map_err(|source| LoaderError::Resource {
            path: directory.to_owned(),
            source,
        })?;
    let mut values = BTreeMap::new();
    for path in paths {
        let sourced = load_document(resource, &path, kind)?;
        let definition_id = id(&sourced.value).clone();
        match values.entry(definition_id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(sourced);
            }
            Entry::Occupied(entry) => {
                return Err(LoaderError::DuplicateId {
                    kind: kind.label(),
                    id: definition_id,
                    first: entry.get().source.clone(),
                    second: sourced.source,
                });
            }
        }
    }
    Ok(values)
}

fn load_document<T>(
    resource: &impl TraditionResource,
    path: &str,
    kind: SchemaKind,
) -> Result<Sourced<T>, LoaderError>
where
    T: DeserializeOwned,
{
    let bytes = resource
        .read(path)
        .map_err(|source| LoaderError::Resource {
            path: path.to_owned(),
            source,
        })?;
    let value: Value =
        serde_yaml_ng::from_slice(&bytes).map_err(|error| LoaderError::MalformedYaml {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
    validate_value(kind, path, &value)?;
    let value = serde_json::from_value(value).map_err(|error| LoaderError::Deserialize {
        path: path.to_owned(),
        kind: kind.label(),
        message: error.to_string(),
    })?;
    Ok(Sourced {
        source: path.to_owned(),
        value,
    })
}

fn validate_references(pack: &LoadedPack) -> Result<(), LoaderError> {
    validate_service_shapes(pack)?;
    validate_authority_graph(pack)?;
    validate_resource_references(pack)?;
    validate_observance_dates(pack)?;
    validate_rule_references(pack)
}

fn validate_resource_references(pack: &LoadedPack) -> Result<(), LoaderError> {
    for sourced in pack.resources.values() {
        for authority in &sourced.value.authority {
            if !pack.authorities.contains_key(authority) {
                return Err(LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: sourced.value.id.clone(),
                    kind: "authority",
                    id: authority.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_authority_graph(pack: &LoadedPack) -> Result<(), LoaderError> {
    for sourced in pack.authorities.values() {
        let authority = &sourced.value;
        for source_id in &authority.sources {
            if source_id == &authority.id {
                return Err(LoaderError::Schema {
                    path: sourced.source.clone(),
                    kind: "authority",
                    message: format!("scoped claim '{}' cannot cite itself", authority.id),
                });
            }
            let source =
                pack.authorities
                    .get(source_id)
                    .ok_or_else(|| LoaderError::UnknownReference {
                        path: sourced.source.clone(),
                        owner: authority.id.clone(),
                        kind: "authority source",
                        id: source_id.clone(),
                    })?;
            if source.value.category != AuthorityCategory::Source {
                return Err(LoaderError::Schema {
                    path: sourced.source.clone(),
                    kind: "authority",
                    message: format!(
                        "scoped claim '{}' must reference a source authority; '{}' is {:?}",
                        authority.id, source_id, source.value.category
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_service_shapes(pack: &LoadedPack) -> Result<(), LoaderError> {
    for sourced in pack.services.values() {
        let mut sections = BTreeMap::<&str, &str>::new();
        for section in &sourced.value.sections {
            if sections.insert(&section.id, &sourced.source).is_some() {
                return Err(LoaderError::DuplicateNestedId {
                    path: sourced.source.clone(),
                    kind: "section",
                    id: section.id.clone(),
                    owner: sourced.value.id.clone(),
                });
            }
            let mut slots = BTreeSet::<&str>::new();
            for slot in &section.slots {
                if !slots.insert(&slot.id) {
                    return Err(LoaderError::DuplicateNestedId {
                        path: sourced.source.clone(),
                        kind: "slot",
                        id: slot.id.clone(),
                        owner: format!("{}:{}", sourced.value.id, section.id),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_observance_dates(pack: &LoadedPack) -> Result<(), LoaderError> {
    for sourced in pack.observances.values() {
        for authority in &sourced.value.authority {
            if !pack.authorities.contains_key(authority) {
                return Err(LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: sourced.value.id.clone(),
                    kind: "authority",
                    id: authority.clone(),
                });
            }
        }
        for (service_id, appointments) in &sourced.value.appointments {
            if !pack.services.contains_key(service_id) {
                return Err(LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: sourced.value.id.clone(),
                    kind: "appointment service",
                    id: service_id.clone(),
                });
            }
            for (role, resource_ids) in appointments {
                for resource_id in resource_ids.as_slice() {
                    let resource = pack.resources.get(resource_id).ok_or_else(|| {
                        LoaderError::UnknownReference {
                            path: sourced.source.clone(),
                            owner: sourced.value.id.clone(),
                            kind: "liturgical resource",
                            id: resource_id.clone(),
                        }
                    })?;
                    if resource.value.role != *role {
                        return Err(LoaderError::Schema {
                            path: sourced.source.clone(),
                            kind: "observance",
                            message: format!(
                                "appointment role '{role}' does not match resource '{}' role '{}'",
                                resource.value.id, resource.value.role
                            ),
                        });
                    }
                }
            }
        }
        if let ObservanceDate::Fixed { fixed } = &sourced.value.date {
            let max_day = match fixed.month {
                2 => 29,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            if fixed.day > max_day {
                return Err(LoaderError::InvalidFixedDate {
                    path: sourced.source.clone(),
                    id: sourced.value.id.clone(),
                    month: fixed.month,
                    day: fixed.day,
                });
            }
        }
    }
    Ok(())
}

fn validate_rule_references(pack: &LoadedPack) -> Result<(), LoaderError> {
    for sourced in pack.rules.values() {
        let rule = &sourced.value;
        let service_id = rule
            .when
            .service
            .as_deref()
            .ok_or_else(|| LoaderError::Schema {
                path: sourced.source.clone(),
                kind: "rule",
                message: "when.service is required".to_owned(),
            })?;
        let service =
            pack.services
                .get(service_id)
                .ok_or_else(|| LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: rule.id.clone(),
                    kind: "service",
                    id: service_id.to_owned(),
                })?;

        if let Some(unless_service) = rule
            .unless
            .as_ref()
            .and_then(|unless| unless.service.as_ref())
            && !pack.services.contains_key(unless_service)
        {
            return Err(LoaderError::UnknownReference {
                path: sourced.source.clone(),
                owner: rule.id.clone(),
                kind: "service",
                id: unless_service.clone(),
            });
        }

        for authority in &rule.authority {
            if !pack.authorities.contains_key(authority) {
                return Err(LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: rule.id.clone(),
                    kind: "authority",
                    id: authority.clone(),
                });
            }
        }

        for emission in &rule.emit {
            if emission.appointment.is_some() && rule.when.observance.is_none() {
                return Err(LoaderError::Schema {
                    path: sourced.source.clone(),
                    kind: "rule",
                    message: "appointment emissions require when.observance".to_owned(),
                });
            }
            let section = service
                .value
                .sections
                .iter()
                .find(|section| section.id == emission.section)
                .ok_or_else(|| LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: rule.id.clone(),
                    kind: "section",
                    id: emission.section.clone(),
                })?;
            if !section.slots.iter().any(|slot| slot.id == emission.slot) {
                return Err(LoaderError::UnknownReference {
                    path: sourced.source.clone(),
                    owner: rule.id.clone(),
                    kind: "slot",
                    id: format!("{}:{}", emission.section, emission.slot),
                });
            }
        }
    }
    Ok(())
}

fn ensure_safe_relative(path: &str) -> Result<(), ResourceError> {
    if path.is_empty()
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ResourceError::UnsafePath(path.to_owned()));
    }
    Ok(())
}

fn normalize_resource_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_owned()
}

fn has_yaml_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use typikon_schema::{
        AUTHORITY_SCHEMA, FFI_RESPONSE_SCHEMA, OBSERVANCE_SCHEMA, PACK_SCHEMA, REQUEST_SCHEMA,
        RESOURCE_BUNDLE_SCHEMA, RESOURCE_SCHEMA, RULE_SCHEMA, SERVICE_SCHEMA,
    };

    #[test]
    fn memory_resource_rejects_parent_traversal() {
        let resource = MemoryResource::default();
        assert!(matches!(
            resource.read("../secret"),
            Err(ResourceError::UnsafePath(_))
        ));
    }

    #[test]
    fn constants_match_schema_contract() {
        let expectations = [
            (SchemaKind::Pack, PACK_SCHEMA),
            (SchemaKind::Service, SERVICE_SCHEMA),
            (SchemaKind::Observance, OBSERVANCE_SCHEMA),
            (SchemaKind::Rule, RULE_SCHEMA),
            (SchemaKind::Authority, AUTHORITY_SCHEMA),
            (SchemaKind::Resource, RESOURCE_SCHEMA),
            (SchemaKind::FfiResponse, FFI_RESPONSE_SCHEMA),
            (SchemaKind::Request, REQUEST_SCHEMA),
            (SchemaKind::ResourceBundle, RESOURCE_BUNDLE_SCHEMA),
        ];
        for (kind, expected) in expectations {
            let schema: Value = serde_json::from_str(kind.document_schema()).unwrap();
            let actual = schema["properties"]["schema"]["const"].as_str();
            assert_eq!(actual, Some(expected));
        }
    }

    #[test]
    fn authority_categories_enforce_distinct_evidence_shapes() {
        let source = serde_json::json!({
            "schema": AUTHORITY_SCHEMA,
            "id": "source",
            "title": "Source",
            "category": "source",
            "kind": "authoritative",
            "reference": { "url": "https://example.test/source" }
        });
        validate_value(SchemaKind::Authority, "source", &source).unwrap();

        let claim = serde_json::json!({
            "schema": AUTHORITY_SCHEMA,
            "id": "claim",
            "title": "Claim",
            "category": "scoped_claim",
            "kind": "authoritative",
            "sources": ["source"],
            "claim": "A claim with explicit scope."
        });
        validate_value(SchemaKind::Authority, "claim", &claim).unwrap();

        let witness = serde_json::json!({
            "schema": AUTHORITY_SCHEMA,
            "id": "witness",
            "title": "Witness",
            "category": "dated_witness",
            "kind": "observed_behavior",
            "locator": { "liturgical_date": "2026-07-26" },
            "reference": { "url": "https://example.test/witness" }
        });
        validate_value(SchemaKind::Authority, "witness", &witness).unwrap();

        let mut missing_date = witness;
        missing_date["locator"] = serde_json::json!({ "service": "Vespers" });
        assert!(validate_value(SchemaKind::Authority, "witness", &missing_date).is_err());
    }
}

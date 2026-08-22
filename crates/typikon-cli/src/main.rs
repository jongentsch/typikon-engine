use std::error::Error;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::json;
use typikon_core::Engine;
use typikon_loader::{DirectoryResource, SchemaKind, load_pack, validate_value};
use typikon_schema::{CompileServiceRequest, REQUEST_SCHEMA};

#[derive(Debug, Parser)]
#[command(
    name = "typikon",
    version,
    about = "Validate and evaluate Typikon packs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Load and validate an external tradition pack.
    Validate {
        /// Directory containing pack.yaml.
        pack: PathBuf,
    },
    /// Compile the service fragment selected by the request context.
    CompileService {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        date: String,
        #[arg(long)]
        service: String,
        /// Optional assertion checked against the calculated tradition tone.
        #[arg(long)]
        tone: Option<String>,
        /// Optional assertion checked against the calculated liturgical phase.
        #[arg(long)]
        phase: Option<String>,
        /// Explicit context observance ID; repeat for multiple observances.
        #[arg(long = "observance")]
        observances: Vec<String>,
    },
    /// Print one validated rule and its referenced authority records.
    InspectRule {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        rule: String,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Validate { pack } => {
            let resource = DirectoryResource::new(pack)?;
            let pack = load_pack(&resource)?;
            let summary = json!({
                "status": "valid",
                "pack": { "id": pack.pack.value.id, "version": pack.pack.value.version },
                "definitions": {
                    "services": pack.services.len(),
                    "observances": pack.observances.len(),
                    "rules": pack.rules.len(),
                    "authorities": pack.authorities.len()
                }
            });
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::CompileService {
            pack,
            date,
            service,
            tone,
            phase,
            observances,
        } => {
            let resource = DirectoryResource::new(pack)?;
            let engine = Engine::new(load_pack(&resource)?);
            let plan = engine.compile_service(CompileServiceRequest {
                schema: REQUEST_SCHEMA.to_owned(),
                civil_date: date,
                service,
                tone,
                phase,
                observances,
            })?;
            let value = serde_json::to_value(&plan)?;
            validate_value(SchemaKind::Plan, "compiled plan", &value)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::InspectRule { pack, rule } => {
            let resource = DirectoryResource::new(pack)?;
            let pack = load_pack(&resource)?;
            let sourced = pack
                .rules
                .get(&rule)
                .ok_or_else(|| format!("unknown rule '{rule}'"))?;
            let authorities = sourced
                .value
                .authority
                .iter()
                .map(|id| &pack.authorities[id].value)
                .collect::<Vec<_>>();
            let result = json!({
                "source": sourced.source,
                "rule": sourced.value,
                "authorities": authorities
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}

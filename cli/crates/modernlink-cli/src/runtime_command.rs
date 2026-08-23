use clap::Subcommand;
use runtime_observer::{
    GenericHttpConnector, ObservationDepth, ReqwestHttpTransport, RuntimeConnector, RuntimeProfile,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Debug, Subcommand)]
pub enum RuntimeCommand {
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Probe {
        #[arg(long)]
        profile: PathBuf,
    },
    Observe {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long, default_value = "metadata", value_parser = ["metadata"])]
        depth: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}
#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    Validate { profile: PathBuf },
}
pub fn run(command: RuntimeCommand) -> Result<(), String> {
    match command {
        RuntimeCommand::Profile {
            command: ProfileCommand::Validate { profile },
        } => {
            let profile = read_profile(&profile)?;
            println!(
                "{}",
                serde_json::json!({"valid": true, "profile_digest": profile.digest().map_err(|error| error.to_string())?})
            );
            Ok(())
        }
        RuntimeCommand::Probe { profile } => {
            let profile = read_profile(&profile)?;
            let connector = GenericHttpConnector::new(
                ReqwestHttpTransport::new().map_err(|error| error.to_string())?,
            );
            let report = connector
                .probe(&profile)
                .map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        RuntimeCommand::Observe {
            profile,
            depth,
            output,
        } => {
            let profile = read_profile(&profile)?;
            let depth: ObservationDepth = depth
                .parse()
                .map_err(|error: runtime_observer::ObservationError| error.to_string())?;
            let connector = GenericHttpConnector::new(
                ReqwestHttpTransport::new().map_err(|error| error.to_string())?,
            );
            let captured_at = format!(
                "unix-seconds:{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| error.to_string())?
                    .as_secs()
            );
            let snapshot = connector
                .observe_metadata(&profile, depth, &captured_at)
                .map_err(|error| error.to_string())?;
            let json = snapshot
                .canonical_json()
                .map_err(|error| error.to_string())?;
            if let Some(output) = output {
                if let Some(parent) = output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                fs::write(&output, json).map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::json!({"snapshot": output, "content_digest": snapshot.content_digest})
                );
            } else {
                print!("{json}");
            }
            Ok(())
        }
    }
}
fn read_profile(path: &PathBuf) -> Result<RuntimeProfile, String> {
    let json = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    RuntimeProfile::from_json(&json).map_err(|error| error.to_string())
}

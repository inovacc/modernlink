use clap::Subcommand;
use runtime_observer::{
    GenericHttpConnector, ObservationDepth, ObservationError, ReqwestHttpTransport,
    RuntimeConnector, RuntimeProfile,
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

pub enum RuntimeCommandError {
    Input(String),
    Network(String),
    Output(String),
    Internal(String),
}

pub fn into_command_error(error: RuntimeCommandError) -> crate::CommandError {
    match error {
        RuntimeCommandError::Input(message) => crate::CommandError::invalid_input(message),
        RuntimeCommandError::Network(message) => crate::CommandError::network(message),
        RuntimeCommandError::Output(message) => crate::CommandError::io(message),
        RuntimeCommandError::Internal(message) => crate::CommandError::internal(message),
    }
}

pub fn run(command: RuntimeCommand) -> Result<(), RuntimeCommandError> {
    match command {
        RuntimeCommand::Profile {
            command: ProfileCommand::Validate { profile },
        } => {
            let profile = read_profile(&profile)?;
            println!(
                "{}",
                serde_json::json!({"valid": true, "profile_digest": profile.digest().map_err(internal)?})
            );
            Ok(())
        }
        RuntimeCommand::Probe { profile } => {
            let profile = read_profile(&profile)?;
            let connector =
                GenericHttpConnector::new(ReqwestHttpTransport::new().map_err(observation_error)?);
            let report = connector.probe(&profile).map_err(observation_error)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(internal)?
            );
            Ok(())
        }
        RuntimeCommand::Observe {
            profile,
            depth,
            output,
        } => {
            let profile = read_profile(&profile)?;
            let depth: ObservationDepth = depth.parse().map_err(observation_error)?;
            let connector =
                GenericHttpConnector::new(ReqwestHttpTransport::new().map_err(observation_error)?);
            let captured_at = format!(
                "unix-seconds:{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| RuntimeCommandError::Internal(error.to_string()))?
                    .as_secs()
            );
            let snapshot = connector
                .observe_metadata(&profile, depth, &captured_at)
                .map_err(observation_error)?;
            let json = snapshot.canonical_json().map_err(observation_error)?;
            if let Some(output) = output {
                if let Some(parent) = output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    fs::create_dir_all(parent)
                        .map_err(|error| RuntimeCommandError::Output(error.to_string()))?;
                }
                fs::write(&output, json)
                    .map_err(|error| RuntimeCommandError::Output(error.to_string()))?;
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
fn read_profile(path: &PathBuf) -> Result<RuntimeProfile, RuntimeCommandError> {
    let json = fs::read_to_string(path).map_err(|error| {
        RuntimeCommandError::Input(format!("cannot read {}: {error}", path.display()))
    })?;
    RuntimeProfile::from_json(&json).map_err(observation_error)
}

fn observation_error(error: ObservationError) -> RuntimeCommandError {
    match error {
        ObservationError::InvalidProfile(message) => RuntimeCommandError::Input(message),
        ObservationError::Transport(message) | ObservationError::Response(message) => {
            RuntimeCommandError::Network(message)
        }
        ObservationError::Serialize(error) => RuntimeCommandError::Internal(error.to_string()),
    }
}

fn internal(error: impl ToString) -> RuntimeCommandError {
    RuntimeCommandError::Internal(error.to_string())
}

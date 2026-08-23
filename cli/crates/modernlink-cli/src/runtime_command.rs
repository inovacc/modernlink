use clap::Subcommand;
use runtime_observer::{
    ConnectorKind, CredentialResolver, GenericHttpConnector, KubernetesConnector, ObservationDepth,
    ObservationError, ReqwestHttpTransport, RuntimeConnector, RuntimeProfile, SshConnector,
    SystemCommandExecutor,
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
        /// Explicit path to an approved installed kubectl or ssh executable.
        #[arg(long)]
        tool: Option<PathBuf>,
    },
    Observe {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long, default_value = "metadata", value_parser = ["metadata"])]
        depth: String,
        #[arg(long)]
        output: Option<PathBuf>,
        /// Explicit path to an approved installed kubectl or ssh executable.
        #[arg(long)]
        tool: Option<PathBuf>,
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
        RuntimeCommand::Probe { profile, tool } => {
            let profile = read_profile(&profile)?;
            resolve_credential_reference(&profile)?;
            let report = probe(&profile, tool.as_deref())?;
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
            tool,
        } => {
            let profile = read_profile(&profile)?;
            resolve_credential_reference(&profile)?;
            let depth: ObservationDepth = depth.parse().map_err(observation_error)?;
            let captured_at = format!(
                "unix-seconds:{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| RuntimeCommandError::Internal(error.to_string()))?
                    .as_secs()
            );
            let snapshot = observe(&profile, tool.as_deref(), depth, &captured_at)?;
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

fn probe(
    profile: &RuntimeProfile,
    tool: Option<&std::path::Path>,
) -> Result<runtime_observer::CapabilityReport, RuntimeCommandError> {
    match profile.kind {
        ConnectorKind::GenericHttp => {
            GenericHttpConnector::new(ReqwestHttpTransport::new().map_err(observation_error)?)
                .probe(profile)
                .map_err(observation_error)
        }
        ConnectorKind::Kubernetes => {
            let executor = SystemCommandExecutor;
            KubernetesConnector::new(
                &executor,
                tool.unwrap_or_else(|| std::path::Path::new("kubectl"))
                    .to_path_buf(),
            )
            .probe(profile)
            .map_err(observation_error)
        }
        ConnectorKind::Ssh => {
            let executor = SystemCommandExecutor;
            SshConnector::new(
                &executor,
                tool.unwrap_or_else(|| std::path::Path::new("ssh"))
                    .to_path_buf(),
            )
            .probe(profile)
            .map_err(observation_error)
        }
        _ => Err(RuntimeCommandError::Input(
            "connector is not available in this runtime command build".to_owned(),
        )),
    }
}

fn observe(
    profile: &RuntimeProfile,
    tool: Option<&std::path::Path>,
    depth: ObservationDepth,
    captured_at: &str,
) -> Result<runtime_observer::ObservationSnapshot, RuntimeCommandError> {
    match profile.kind {
        ConnectorKind::GenericHttp => {
            GenericHttpConnector::new(ReqwestHttpTransport::new().map_err(observation_error)?)
                .observe_metadata(profile, depth, captured_at)
                .map_err(observation_error)
        }
        ConnectorKind::Kubernetes => {
            let executor = SystemCommandExecutor;
            KubernetesConnector::new(
                &executor,
                tool.unwrap_or_else(|| std::path::Path::new("kubectl"))
                    .to_path_buf(),
            )
            .observe_metadata(profile, depth, captured_at)
            .map_err(observation_error)
        }
        ConnectorKind::Ssh => {
            let executor = SystemCommandExecutor;
            SshConnector::new(
                &executor,
                tool.unwrap_or_else(|| std::path::Path::new("ssh"))
                    .to_path_buf(),
            )
            .observe_metadata(profile, depth, captured_at)
            .map_err(observation_error)
        }
        _ => Err(RuntimeCommandError::Input(
            "connector is not available in this runtime command build".to_owned(),
        )),
    }
}

fn resolve_credential_reference(profile: &RuntimeProfile) -> Result<(), RuntimeCommandError> {
    if let Some(reference) = &profile.credential_ref {
        let executor = SystemCommandExecutor;
        let _credential =
            CredentialResolver::resolve(reference, &executor, None).map_err(observation_error)?;
    }
    Ok(())
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
        ObservationError::Transport(message)
        | ObservationError::Response(message)
        | ObservationError::Command(message)
        | ObservationError::Tunnel(message) => RuntimeCommandError::Network(message),
        ObservationError::Credential(message) => RuntimeCommandError::Input(message),
        ObservationError::Serialize(error) => RuntimeCommandError::Internal(error.to_string()),
    }
}

fn internal(error: impl ToString) -> RuntimeCommandError {
    RuntimeCommandError::Internal(error.to_string())
}

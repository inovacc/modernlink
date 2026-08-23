use std::{env, fs, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};
use modernlink_analyzer::analyze_repository;
use sha2::{Digest, Sha256};

mod runtime_command;
use runtime_command::RuntimeCommand;

#[derive(Debug, Parser)]
#[command(
    name = "modernlink",
    version,
    about = "Deterministic modernization analyzer"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze a repository and write a deterministic evidence graph.
    Analyze {
        /// Repository root to analyze.
        repository: PathBuf,
        /// JSON report path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Manage the thin agent-plugin binding.
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Validate and observe an operator-authorized runtime target.
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    /// Bind an installed plugin to one exact ModernLink binary.
    Bind {
        #[arg(long)]
        plugin_root: PathBuf,
        #[arg(long)]
        binary: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let diagnostic = serde_json::json!({
                "code": error.code,
                "message": error.message,
            });
            eprintln!("{diagnostic}");
            ExitCode::from(error.exit_code)
        }
    }
}

fn run(cli: Cli) -> Result<(), CommandError> {
    match cli.command {
        Command::Analyze { repository, output } => {
            let report = analyze_repository(&repository)
                .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            if let Some(parent) = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent).map_err(|error| {
                    CommandError::io(format!("cannot create output directory: {error}"))
                })?;
            }
            fs::write(&output, json).map_err(|error| {
                CommandError::io(format!("cannot write {}: {error}", output.display()))
            })?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "repository_digest": report.repository_digest,
                    "summary": report.summary,
                })
            );
            Ok(())
        }
        Command::Plugin {
            command:
                PluginCommand::Bind {
                    plugin_root,
                    binary,
                },
        } => bind_plugin(plugin_root, binary),
        Command::Runtime { command } => {
            runtime_command::run(command).map_err(CommandError::invalid_input)
        }
    }
}

fn bind_plugin(plugin_root: PathBuf, binary: PathBuf) -> Result<(), CommandError> {
    if !plugin_root.is_dir() {
        return Err(CommandError::invalid_input(format!(
            "plugin root is not a directory: {}",
            plugin_root.display()
        )));
    }
    let binary = binary.canonicalize().map_err(|error| {
        CommandError::io(format!("cannot resolve {}: {error}", binary.display()))
    })?;
    if !binary.is_file() {
        return Err(CommandError::invalid_input(format!(
            "binary path is not a file: {}",
            binary.display()
        )));
    }
    let bytes = fs::read(&binary)
        .map_err(|error| CommandError::io(format!("cannot read {}: {error}", binary.display())))?;
    let pointer = serde_json::json!({
        "schema_version": "modernlink.binary-pointer/v1",
        "binary_path": binary,
        "binary_sha256": format!("sha256:{}", hex::encode(Sha256::digest(bytes))),
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "analysis_schema": "modernlink.analysis/v1alpha1",
    });
    let config_dir = plugin_root.join("config");
    fs::create_dir_all(&config_dir).map_err(|error| {
        CommandError::io(format!("cannot create {}: {error}", config_dir.display()))
    })?;
    let pointer_path = config_dir.join("binary-pointer.json");
    let mut json = serde_json::to_string_pretty(&pointer)
        .map_err(|error| CommandError::internal(error.to_string()))?;
    json.push('\n');
    fs::write(&pointer_path, json).map_err(|error| {
        CommandError::io(format!("cannot write {}: {error}", pointer_path.display()))
    })?;
    println!(
        "{}",
        serde_json::json!({"pointer": pointer_path, "binary": binary})
    );
    Ok(())
}

struct CommandError {
    code: &'static str,
    message: String,
    exit_code: u8,
}

impl CommandError {
    fn invalid_input(message: String) -> Self {
        Self {
            code: "MLK-INPUT-001",
            message,
            exit_code: 1,
        }
    }

    fn io(message: String) -> Self {
        Self {
            code: "MLK-IO-001",
            message,
            exit_code: 2,
        }
    }

    fn internal(message: String) -> Self {
        Self {
            code: "MLK-INTERNAL-001",
            message,
            exit_code: 4,
        }
    }
}

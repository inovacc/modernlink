use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand, ValueEnum};
use git::{HistoryOptions, MailmapMode, RefScope, collect_history_cached};
use harness::HarnessRegistry;
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
    /// Create or refresh local ModernLink workspace metadata without touching user-authored harness files.
    Setup {
        /// Repository root; defaults to the current directory.
        #[arg(default_value = ".")]
        repository: PathBuf,
        /// Comma-separated harness IDs, `all`, or `none`.
        #[arg(long, default_value = "none")]
        tools: String,
        /// Report the managed paths and detected harnesses without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Replace the existing ModernLink-owned workspace manifest only.
        #[arg(long)]
        force: bool,
    },
    /// Recover and display a modernization lifecycle snapshot from its journal.
    Status {
        /// Append-only lifecycle journal JSON Lines path.
        #[arg(long)]
        journal: PathBuf,
        /// Stable identifier for the modernization run.
        #[arg(long)]
        run_id: String,
    },
    /// Assess target-runtime risks from a shared evidence graph.
    Compatibility {
        /// Shared evidence graph JSON path.
        #[arg(long)]
        evidence: PathBuf,
        /// Target Java runtime major version.
        #[arg(long)]
        target: u16,
        /// Compatibility assessment JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Identify evidence-backed modernization seams from a shared evidence graph.
    Seams {
        /// Shared evidence graph JSON path.
        #[arg(long)]
        evidence: PathBuf,
        /// Modernization seam report JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Infer architectural layers and candidate contexts from a shared evidence graph.
    Architecture {
        /// Shared evidence graph JSON path.
        #[arg(long)]
        evidence: PathBuf,
        /// Structural inference JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Collect deterministic repository facts into the shared evidence graph.
    Inspect {
        /// Repository root to inspect.
        repository: PathBuf,
        /// Shared evidence graph JSON output path.
        #[arg(long, short)]
        output: PathBuf,
        /// Merge local Git evolution evidence when the repository has Git history.
        #[arg(long)]
        history: bool,
    },
    /// Analyze a repository and write a deterministic evidence graph.
    Analyze {
        /// Repository root to analyze.
        repository: PathBuf,
        /// JSON report path.
        #[arg(long, short)]
        output: PathBuf,
        /// Also write deterministic Git history evidence beside the analysis report.
        #[arg(long)]
        history: bool,
    },
    /// Collect local, deterministic Git evolution evidence for a repository.
    History {
        /// Git repository root to inspect.
        repository: PathBuf,
        /// JSON report path.
        #[arg(long, short)]
        output: PathBuf,
        /// Refs to include: all, head, local, or one explicit ref name.
        #[arg(long, default_value = "all")]
        refs: String,
        /// Maximum unique commits to collect.
        #[arg(long)]
        max_commits: Option<usize>,
        /// Maximum changed paths per commit before co-change expansion is skipped.
        #[arg(long)]
        max_cochange_paths: Option<usize>,
        /// Whether to apply the repository mailmap when identity support is available.
        #[arg(long, value_enum, default_value_t = MailmapArgument::Off)]
        mailmap: MailmapArgument,
    },
    /// Manage the thin agent-plugin binding.
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Inspect the descriptor-driven AI harness registry.
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
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

#[derive(Debug, Subcommand)]
enum HarnessCommand {
    /// List bundled harness descriptors as structured JSON.
    List,
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
        Command::Setup {
            repository,
            tools,
            dry_run,
            force,
        } => setup_workspace(repository, tools, dry_run, force),
        Command::Status { journal, run_id } => {
            let snapshot = state::recover_journal(&run_id, &journal).map_err(|error| {
                CommandError::invalid_input(format!(
                    "cannot recover lifecycle journal {}: {error}",
                    journal.display()
                ))
            })?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": "modernlink.lifecycle-status/v1alpha1",
                    "journal": journal,
                    "snapshot": snapshot,
                })
            );
            Ok(())
        }
        Command::Compatibility {
            evidence,
            target,
            output,
        } => {
            if target < 8 {
                return Err(CommandError::invalid_input(
                    "--target must be a Java major version of at least 8".to_owned(),
                ));
            }
            let graph = read_evidence_graph(&evidence)?;
            let report = inference::assess_compatibility(&graph, target)
                .map_err(|error| CommandError::internal(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            ensure_report_absent(&output)?;
            write_report(&output, json)?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "schema_version": report.schema_version,
                    "summary": {
                        "target_version": report.target_version,
                        "findings": report.findings.len(),
                    },
                })
            );
            Ok(())
        }
        Command::Seams { evidence, output } => {
            let graph = read_evidence_graph(&evidence)?;
            let report = inference::infer_seams(&graph)
                .map_err(|error| CommandError::internal(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            ensure_report_absent(&output)?;
            write_report(&output, json)?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "schema_version": report.schema_version,
                    "summary": { "seams": report.seams.len() },
                })
            );
            Ok(())
        }
        Command::Architecture { evidence, output } => {
            let graph = read_evidence_graph(&evidence)?;
            let report = inference::infer_structure(&graph)
                .map_err(|error| CommandError::internal(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            ensure_report_absent(&output)?;
            write_report(&output, json)?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "schema_version": report.schema_version,
                    "summary": {
                        "layers": report.layers.len(),
                        "candidate_contexts": report.contexts.len(),
                    },
                })
            );
            Ok(())
        }
        Command::Inspect {
            repository,
            output,
            history,
        } => {
            let report = analyze_repository(&repository)
                .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let graph = report
                .to_evidence_graph()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            let graph = if history {
                let history = collect_history_cached(&repository, &HistoryOptions::all())
                    .map_err(|error| CommandError::invalid_input(error.to_string()))?;
                graph
                    .merge(
                        history
                            .to_evidence_graph()
                            .map_err(|error| CommandError::internal(error.to_string()))?,
                    )
                    .map_err(|error| CommandError::internal(error.to_string()))?
            } else {
                graph
            };
            let json = graph
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            ensure_report_absent(&output)?;
            write_report(&output, json)?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "schema_version": graph.schema_version,
                    "summary": {
                        "evidence": graph.evidence.len(),
                        "nodes": graph.nodes.len(),
                        "edges": graph.edges.len(),
                        "claims": graph.claims.len(),
                    },
                })
            );
            Ok(())
        }
        Command::Analyze {
            repository,
            output,
            history,
        } => {
            let report = analyze_repository(&repository)
                .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            let history_output = output
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("git-history.json");
            if history && history_output == output {
                return Err(CommandError::invalid_input(
                    "--history requires an analysis output path other than git-history.json"
                        .to_owned(),
                ));
            }
            let history_report = history
                .then(|| collect_history_cached(&repository, &HistoryOptions::all()))
                .transpose()
                .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let history_json = history_report
                .as_ref()
                .map(|report| report.canonical_json())
                .transpose()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            ensure_report_absent(&output)?;
            if history_json.is_some() {
                ensure_report_absent(&history_output)?;
            }
            write_report(&output, json)?;
            if let Some(history_json) = history_json {
                write_report(&history_output, history_json)?;
            }
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "repository_digest": report.repository_digest,
                    "summary": report.summary,
                    "history_report": history.then_some(history_output),
                })
            );
            Ok(())
        }
        Command::History {
            repository,
            output,
            refs,
            max_commits,
            max_cochange_paths,
            mailmap,
        } => {
            let options = history_options(refs, max_commits, max_cochange_paths, mailmap)?;
            let report = collect_history_cached(&repository, &options)
                .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let json = report
                .canonical_json()
                .map_err(|error| CommandError::internal(error.to_string()))?;
            write_report(&output, json)?;
            println!(
                "{}",
                serde_json::json!({
                    "report": output,
                    "repository_digest": report.repository.repository_digest,
                    "summary": {
                        "commits": report.commits.len(),
                        "path_changes": report.path_changes.len(),
                        "co_changes": report.co_changes.len(),
                        "completeness": report.completeness,
                    },
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
        Command::Harness {
            command: HarnessCommand::List,
        } => {
            let registry = HarnessRegistry::builtin();
            println!("{}", serde_json::json!({ "harnesses": registry.all() }));
            Ok(())
        }
        Command::Runtime { command } => {
            runtime_command::run(command).map_err(runtime_command::into_command_error)
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct WorkspaceManifest {
    schema_version: &'static str,
    selected_harnesses: Vec<String>,
    detected_harnesses: Vec<String>,
    managed_paths: Vec<&'static str>,
    adapter_installation: &'static str,
}

fn setup_workspace(
    repository: PathBuf,
    tools: String,
    dry_run: bool,
    force: bool,
) -> Result<(), CommandError> {
    let repository = repository.canonicalize().map_err(|error| {
        CommandError::io(format!(
            "cannot resolve repository {}: {error}",
            repository.display()
        ))
    })?;
    if !repository.is_dir() {
        return Err(CommandError::invalid_input(format!(
            "repository path is not a directory: {}",
            repository.display()
        )));
    }
    let registry = HarnessRegistry::builtin();
    let selected_harnesses = select_harnesses(&registry, &tools)?;
    let detected_harnesses = registry
        .all()
        .iter()
        .filter(|definition| {
            definition
                .detection_markers
                .iter()
                .any(|marker| repository.join(marker).exists())
        })
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    let manifest = WorkspaceManifest {
        schema_version: "modernlink.workspace/v1alpha1",
        selected_harnesses,
        detected_harnesses,
        managed_paths: vec![
            ".modernlink/workspace.json",
            ".modernlink/cache/",
            ".modernlink/local/",
            ".modernlink/state/",
        ],
        adapter_installation: "not-attempted: no harness adapter has an approved ownership/install contract",
    };
    let manifest_path = repository.join(".modernlink").join("workspace.json");
    if manifest_path.exists() && !force {
        return Err(CommandError::io(format!(
            "refusing to overwrite ModernLink workspace manifest {}; rerun with --force to replace only this managed file",
            manifest_path.display()
        )));
    }
    if !dry_run {
        for path in ["cache", "local", "state"] {
            fs::create_dir_all(repository.join(".modernlink").join(path)).map_err(|error| {
                CommandError::io(format!(
                    "cannot create ModernLink workspace directory: {error}"
                ))
            })?;
        }
        let mut json = serde_json::to_string_pretty(&manifest)
            .map_err(|error| CommandError::internal(error.to_string()))?;
        json.push('\n');
        fs::write(&manifest_path, json).map_err(|error| {
            CommandError::io(format!(
                "cannot write ModernLink workspace manifest {}: {error}",
                manifest_path.display()
            ))
        })?;
    }
    println!(
        "{}",
        serde_json::json!({
            "schema_version": "modernlink.setup-result/v1alpha1",
            "repository": repository,
            "dry_run": dry_run,
            "workspace_manifest": manifest_path,
            "workspace": manifest,
        })
    );
    Ok(())
}

fn select_harnesses(registry: &HarnessRegistry, tools: &str) -> Result<Vec<String>, CommandError> {
    let requested = tools.trim();
    if requested == "none" {
        return Ok(Vec::new());
    }
    if requested == "all" {
        return Ok(registry.all().iter().map(|item| item.id.clone()).collect());
    }
    let mut selected = requested
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(CommandError::invalid_input(
            "--tools must be `none`, `all`, or one or more comma-separated harness IDs".to_owned(),
        ));
    }
    selected.sort();
    selected.dedup();
    for id in &selected {
        if registry.get(id).is_none() {
            return Err(CommandError::invalid_input(format!(
                "unknown harness `{id}`; run `modernlink harness list` to inspect available IDs"
            )));
        }
    }
    Ok(selected)
}

fn read_evidence_graph(path: &Path) -> Result<model::EvidenceGraph, CommandError> {
    let input = fs::read_to_string(path)
        .map_err(|error| CommandError::io(format!("cannot read {}: {error}", path.display())))?;
    model::EvidenceGraph::from_json(&input)
        .map_err(|error| CommandError::invalid_input(error.to_string()))
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MailmapArgument {
    Off,
    Repo,
}

fn history_options(
    refs: String,
    max_commits: Option<usize>,
    max_cochange_paths: Option<usize>,
    mailmap: MailmapArgument,
) -> Result<HistoryOptions, CommandError> {
    let mut options = HistoryOptions::all();
    options.ref_scope = match refs.as_str() {
        "all" => RefScope::All,
        "head" => RefScope::Head,
        "local" => RefScope::Local,
        _ => RefScope::Explicit(refs),
    };
    if let Some(max_commits) = max_commits {
        if max_commits == 0 {
            return Err(CommandError::invalid_input(
                "--max-commits must be greater than zero".to_owned(),
            ));
        }
        options.max_commits = max_commits;
    }
    if let Some(max_cochange_paths) = max_cochange_paths {
        if max_cochange_paths == 0 {
            return Err(CommandError::invalid_input(
                "--max-cochange-paths must be greater than zero".to_owned(),
            ));
        }
        options.max_cochange_paths = max_cochange_paths;
    }
    options.mailmap = match mailmap {
        MailmapArgument::Off => MailmapMode::Off,
        MailmapArgument::Repo => MailmapMode::Repository,
    };
    Ok(options)
}

fn write_report(output: &Path, json: String) -> Result<(), CommandError> {
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::io(format!("cannot create output directory: {error}"))
        })?;
    }
    let parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        CommandError::io(format!(
            "cannot create temporary report beside {}: {error}",
            output.display()
        ))
    })?;
    temporary.write_all(json.as_bytes()).map_err(|error| {
        CommandError::io(format!(
            "cannot write temporary report for {}: {error}",
            output.display()
        ))
    })?;
    temporary.flush().map_err(|error| {
        CommandError::io(format!(
            "cannot flush temporary report for {}: {error}",
            output.display()
        ))
    })?;
    temporary.persist_noclobber(output).map_err(|error| {
        CommandError::io(format!(
            "refusing to overwrite existing report {}: {}",
            output.display(),
            error.error
        ))
    })?;
    Ok(())
}

fn ensure_report_absent(output: &Path) -> Result<(), CommandError> {
    if output.exists() {
        return Err(CommandError::io(format!(
            "refusing to overwrite existing report {}",
            output.display()
        )));
    }
    Ok(())
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

    fn network(message: String) -> Self {
        Self {
            code: "MLK-NETWORK-001",
            message,
            exit_code: 3,
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

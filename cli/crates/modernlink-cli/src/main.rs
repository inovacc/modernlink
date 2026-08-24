use std::{
    env, fs,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::MultiSelect;
use git::{HistoryOptions, MailmapMode, RefScope, collect_history_cached};
use harness::HarnessRegistry;
use include_dir::{Dir, DirEntry, include_dir};
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
    /// Check static migration-plan integrity and declared approval gates.
    Verify {
        /// Migration plan JSON path.
        #[arg(long)]
        plan: PathBuf,
        /// Verification report JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// List annotation-backed static boundary candidates from a shared evidence graph.
    Boundaries {
        /// Shared evidence graph JSON path.
        #[arg(long)]
        evidence: PathBuf,
        /// Boundary report JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Derive an evidence-linked evolution x-ray from a local Git history artifact.
    Evolution {
        /// ModernLink Git history JSON path.
        #[arg(long)]
        history: PathBuf,
        /// Evolution x-ray JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Inspect local CLI, repository, workspace, Git, and harness prerequisites without changing them.
    Doctor {
        /// Repository root; defaults to the current directory.
        #[arg(default_value = ".")]
        repository: PathBuf,
    },
    /// Build an evidence-linked migration dependency DAG from seam and compatibility reports.
    Plan {
        /// Modernization seam report JSON path.
        #[arg(long)]
        seams: PathBuf,
        /// Compatibility assessment report JSON path.
        #[arg(long)]
        compatibility: PathBuf,
        /// Migration plan JSON output path.
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Create or refresh local ModernLink workspace metadata without touching user-authored harness files.
    Setup {
        /// Repository root; defaults to the current directory.
        #[arg(default_value = ".")]
        repository: PathBuf,
        /// Comma-separated harness IDs, `all`, or `none`.
        #[arg(long)]
        tools: Option<String>,
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
    /// Advance an append-only modernization lifecycle journal by exactly one phase.
    Lifecycle {
        #[command(subcommand)]
        command: LifecycleCommand,
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
    /// Materialize the canonical plugin bundle into an explicit new directory.
    Install {
        /// Empty destination directory to create for the ModernLink-owned plugin bundle.
        #[arg(long)]
        destination: PathBuf,
    },
    /// Bind an installed plugin to one exact ModernLink binary.
    Bind {
        #[arg(long)]
        plugin_root: PathBuf,
        #[arg(long)]
        binary: PathBuf,
    },
}

static PLUGIN_SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/skills");
static PLUGIN_AGENTS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/agents");
static PLUGIN_COMMANDS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/commands");
static PLUGIN_HARNESSES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/harnesses");
static PLUGIN_CODEX: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/.codex-plugin");
static PLUGIN_CLAUDE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugin/.claude-plugin");

#[derive(Debug, Subcommand)]
enum HarnessCommand {
    /// List bundled harness descriptors as structured JSON.
    List,
}

#[derive(Debug, Subcommand)]
enum LifecycleCommand {
    /// Append the sole valid next lifecycle transition.
    Advance {
        /// Append-only lifecycle journal JSON Lines path.
        #[arg(long)]
        journal: PathBuf,
        /// Stable identifier for the modernization run.
        #[arg(long)]
        run_id: String,
        /// Explicitly approve a protected transition into MODERNIZE, CUTOVER, or DETACH.
        #[arg(long)]
        approve: bool,
        /// SHA-256 artifact digest associated with this transition; repeat as needed.
        #[arg(long = "artifact")]
        artifact_hashes: Vec<String>,
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
        Command::Verify { plan, output } => {
            let plan = read_json::<inference::MigrationPlan>(&plan, "migration plan")?;
            let report = inference::verify_plan(&plan);
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
                    "summary": { "checks": report.checks.len(), "gaps": report.checks.iter().filter(|check| check.status == "GAP").count() },
                })
            );
            Ok(())
        }
        Command::Boundaries { evidence, output } => {
            let graph = read_evidence_graph(&evidence)?;
            let report = inference::infer_boundaries(&graph)
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
                    "summary": { "boundaries": report.boundaries.len() },
                })
            );
            Ok(())
        }
        Command::Evolution { history, output } => {
            let history = read_json::<git::GitHistorySnapshot>(&history, "Git history artifact")?;
            let report = git::xray_history(&history);
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
                    "summary": { "hotspots": report.hotspots.len(), "co_changes": report.co_changes.len(), "contributors": report.contributors.len() },
                })
            );
            Ok(())
        }
        Command::Doctor { repository } => doctor(repository),
        Command::Plan {
            seams,
            compatibility,
            output,
        } => {
            let seams = read_json::<inference::SeamReport>(&seams, "modernization seam report")?;
            let compatibility = read_json::<inference::CompatibilityReport>(
                &compatibility,
                "compatibility assessment report",
            )?;
            let report = inference::plan_migration(&seams, &compatibility);
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
                    "summary": { "target_version": report.target_version, "tasks": report.tasks.len() },
                })
            );
            Ok(())
        }
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
        Command::Lifecycle {
            command:
                LifecycleCommand::Advance {
                    journal,
                    run_id,
                    approve,
                    artifact_hashes,
                },
        } => advance_lifecycle(journal, run_id, approve, artifact_hashes),
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
            command: PluginCommand::Install { destination },
        } => install_plugin(destination),
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
struct DoctorCheck {
    status: &'static str,
    detail: String,
}

fn doctor(repository: PathBuf) -> Result<(), CommandError> {
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
    let binary = env::current_exe().map_err(|error| {
        CommandError::io(format!("cannot resolve current ModernLink binary: {error}"))
    })?;
    let repository_readable = fs::read_dir(&repository).is_ok();
    let git = match git::open_repository(&repository) {
        Ok(_) => DoctorCheck {
            status: "OK",
            detail: "opened through the structured Rust Git engine".to_owned(),
        },
        Err(error) => DoctorCheck {
            status: "UNAVAILABLE",
            detail: format!("not available to ModernLink: {error}"),
        },
    };
    let registry = HarnessRegistry::builtin();
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
    let workspace = repository.join(".modernlink").join("workspace.json");
    let checks = serde_json::json!({
        "cli": DoctorCheck { status: "OK", detail: binary.display().to_string() },
        "repository": DoctorCheck { status: if repository_readable { "OK" } else { "UNAVAILABLE" }, detail: repository.display().to_string() },
        "git": git,
        "workspace": DoctorCheck { status: if workspace.is_file() { "OK" } else { "MISSING" }, detail: workspace.display().to_string() },
        "java": program_check("java", "-version"),
        "maven": program_check("mvn", "--version"),
        "gradle": program_check("gradle", "--version"),
        "harnesses": { "detected": detected_harnesses, "registry": registry.all() },
    });
    println!(
        "{}",
        serde_json::json!({
            "schema_version": "modernlink.doctor/v1alpha1",
            "repository": repository,
            "checks": checks,
        })
    );
    Ok(())
}

fn program_check(program: &str, version_flag: &str) -> DoctorCheck {
    match std::process::Command::new(program)
        .arg(version_flag)
        .output()
    {
        Ok(_) => DoctorCheck {
            status: "AVAILABLE",
            detail: "command launched; version text is intentionally not parsed".to_owned(),
        },
        Err(error) => DoctorCheck {
            status: "UNAVAILABLE",
            detail: format!("cannot launch {program}: {error}"),
        },
    }
}

fn advance_lifecycle(
    journal: PathBuf,
    run_id: String,
    approve: bool,
    mut artifact_hashes: Vec<String>,
) -> Result<(), CommandError> {
    let mut snapshot = if journal.exists() {
        state::recover_journal(&run_id, &journal).map_err(|error| {
            CommandError::invalid_input(format!(
                "cannot recover lifecycle journal {}: {error}",
                journal.display()
            ))
        })?
    } else {
        state::LifecycleSnapshot::new(&run_id)
    };
    let mut event = snapshot
        .next_event(approve)
        .map_err(|error| CommandError::invalid_input(error.to_string()))?;
    artifact_hashes.sort();
    artifact_hashes.dedup();
    event.artifact_hashes = artifact_hashes;
    snapshot
        .apply(&event)
        .map_err(|error| CommandError::invalid_input(error.to_string()))?;
    if let Some(parent) = journal
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::io(format!(
                "cannot create lifecycle journal directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    state::append_event(&journal, &event).map_err(|error| {
        CommandError::io(format!(
            "cannot append lifecycle journal {}: {error}",
            journal.display()
        ))
    })?;
    println!(
        "{}",
        serde_json::json!({
            "schema_version": "modernlink.lifecycle-transition/v1alpha1",
            "journal": journal,
            "event": event,
            "snapshot": snapshot,
        })
    );
    Ok(())
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
    tools: Option<String>,
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
    let selected_harnesses =
        select_setup_harnesses(&registry, tools.as_deref(), &detected_harnesses)?;
    let manifest = WorkspaceManifest {
        schema_version: "modernlink.workspace/v1alpha1",
        selected_harnesses,
        detected_harnesses,
        managed_paths: vec![
            ".modernlink/workspace.json",
            ".modernlink/.gitignore",
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
        let workspace_root = repository.join(".modernlink");
        ensure_workspace_ignore(&workspace_root)?;
        for path in ["cache", "local", "state"] {
            fs::create_dir_all(workspace_root.join(path)).map_err(|error| {
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

const WORKSPACE_IGNORE_RULES: &str =
    "# ModernLink local operational state\ncache/\nlocal/\nstate/\nworkspace.json\n";

fn ensure_workspace_ignore(workspace_root: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(workspace_root).map_err(|error| {
        CommandError::io(format!(
            "cannot create ModernLink workspace root {}: {error}",
            workspace_root.display()
        ))
    })?;
    let ignore_path = workspace_root.join(".gitignore");
    if !ignore_path.exists() {
        return fs::write(&ignore_path, WORKSPACE_IGNORE_RULES).map_err(|error| {
            CommandError::io(format!(
                "cannot write ModernLink workspace ignore file {}: {error}",
                ignore_path.display()
            ))
        });
    }
    let existing = fs::read_to_string(&ignore_path).map_err(|error| {
        CommandError::io(format!(
            "cannot read existing ModernLink workspace ignore file {}: {error}",
            ignore_path.display()
        ))
    })?;
    if ["cache/", "local/", "state/", "workspace.json"]
        .iter()
        .all(|rule| existing.lines().any(|line| line.trim() == *rule))
    {
        return Ok(());
    }
    Err(CommandError::io(format!(
        "refusing to modify existing user-owned {} because it lacks ModernLink local-state ignore rules",
        ignore_path.display()
    )))
}

fn select_setup_harnesses(
    registry: &HarnessRegistry,
    tools: Option<&str>,
    detected: &[String],
) -> Result<Vec<String>, CommandError> {
    if let Some(tools) = tools {
        return select_harnesses(registry, tools);
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(CommandError::invalid_input(
            "setup requires --tools in a non-interactive session (for example, --tools codex,claude or --tools none)".to_owned(),
        ));
    }
    let definitions = registry.all();
    let labels = definitions
        .iter()
        .map(|definition| format!("{} ({})", definition.display_name, definition.id))
        .collect::<Vec<_>>();
    let defaults = definitions
        .iter()
        .map(|definition| detected.contains(&definition.id))
        .collect::<Vec<_>>();
    let selected = MultiSelect::new()
        .with_prompt("Which AI harnesses should ModernLink configure?")
        .items(&labels)
        .defaults(&defaults)
        .interact()
        .map_err(|error| {
            CommandError::io(format!(
                "cannot read interactive harness selection: {error}"
            ))
        })?;
    Ok(selected
        .into_iter()
        .map(|index| definitions[index].id.clone())
        .collect())
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

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T, CommandError> {
    let input = fs::read_to_string(path).map_err(|error| {
        CommandError::io(format!("cannot read {label} {}: {error}", path.display()))
    })?;
    serde_json::from_str(&input).map_err(|error| {
        CommandError::invalid_input(format!("cannot parse {label} {}: {error}", path.display()))
    })
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
    if pointer_path.exists() {
        return Err(CommandError::io(format!(
            "refusing to overwrite existing binary pointer {}; install a fresh bundle or remove only the owned pointer after review",
            pointer_path.display()
        )));
    }
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

fn install_plugin(destination: PathBuf) -> Result<(), CommandError> {
    if destination.exists() {
        return Err(CommandError::io(format!(
            "refusing to install into existing path {}; choose a new explicit destination",
            destination.display()
        )));
    }
    fs::create_dir_all(&destination).map_err(|error| {
        CommandError::io(format!(
            "cannot create plugin destination {}: {error}",
            destination.display()
        ))
    })?;
    let result = (|| {
        copy_embedded_dir(&PLUGIN_SKILLS, &destination.join("skills"))?;
        copy_embedded_dir(&PLUGIN_AGENTS, &destination.join("agents"))?;
        copy_embedded_dir(&PLUGIN_COMMANDS, &destination.join("commands"))?;
        copy_embedded_dir(&PLUGIN_HARNESSES, &destination.join("harnesses"))?;
        copy_embedded_dir(&PLUGIN_CODEX, &destination.join(".codex-plugin"))?;
        copy_embedded_dir(&PLUGIN_CLAUDE, &destination.join(".claude-plugin"))?;
        let config = destination.join("config");
        fs::create_dir_all(&config)
            .map_err(|error| CommandError::io(format!("cannot create plugin config: {error}")))?;
        fs::write(
            config.join("binary-pointer.schema.json"),
            include_bytes!("../../../plugin/config/binary-pointer.schema.json"),
        )
        .map_err(|error| CommandError::io(format!("cannot write pointer schema: {error}")))?;
        fs::write(config.join(".gitignore"), "binary-pointer.json\n").map_err(|error| {
            CommandError::io(format!("cannot write pointer ignore rule: {error}"))
        })?;
        fs::write(
            destination.join("WORKFLOWS.md"),
            include_bytes!("../../../plugin/WORKFLOWS.md"),
        )
        .map_err(|error| CommandError::io(format!("cannot write plugin workflow: {error}")))?;
        Ok(())
    })();
    result?;
    println!(
        "{}",
        serde_json::json!({
            "schema_version": "modernlink.plugin-install/v1alpha1",
            "plugin_root": destination,
            "binary_pointer": "not-created; run modernlink plugin bind with this plugin_root and the intended binary",
            "harness_materialization": "not-attempted; adapter paths require an explicit harness ownership contract",
        })
    );
    Ok(())
}

fn copy_embedded_dir(source: &Dir<'_>, destination: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(destination).map_err(|error| {
        CommandError::io(format!(
            "cannot create embedded plugin directory {}: {error}",
            destination.display()
        ))
    })?;
    for entry in source.entries() {
        match entry {
            DirEntry::Dir(directory) => {
                let relative = directory
                    .path()
                    .strip_prefix(source.path())
                    .map_err(|error| CommandError::internal(error.to_string()))?;
                copy_embedded_dir(directory, &destination.join(relative))?;
            }
            DirEntry::File(file) => {
                let relative = file
                    .path()
                    .strip_prefix(source.path())
                    .map_err(|error| CommandError::internal(error.to_string()))?;
                let output = destination.join(relative);
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        CommandError::io(format!("cannot create plugin file parent: {error}"))
                    })?;
                }
                fs::write(&output, file.contents()).map_err(|error| {
                    CommandError::io(format!(
                        "cannot write plugin file {}: {error}",
                        output.display()
                    ))
                })?;
            }
        }
    }
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

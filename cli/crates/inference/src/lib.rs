use std::collections::BTreeMap;

use model::{ClaimState, EvidenceGraph, GraphError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STRUCTURE_SCHEMA_VERSION: &str = "modernlink.structure/v1alpha1";
pub const COMPATIBILITY_SCHEMA_VERSION: &str = "modernlink.compatibility/v1alpha1";

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("cannot use invalid evidence graph: {0}")]
    Graph(#[from] GraphError),
    #[error("cannot serialize structural inference: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureReport {
    pub schema_version: String,
    pub layers: Vec<LayerInference>,
    pub contexts: Vec<CandidateContext>,
}

impl StructureReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .layers
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        normalized
            .contexts
            .sort_by(|left, right| left.name.cmp(&right.name));
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerInference {
    pub node_id: String,
    pub layer: String,
    pub state: ClaimState,
    pub confidence_percent: u8,
    pub evidence_ids: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateContext {
    pub name: String,
    pub state: ClaimState,
    pub confidence_percent: u8,
    pub evidence_ids: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeamReport {
    pub schema_version: String,
    pub seams: Vec<ModernizationSeam>,
}

impl SeamReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .seams
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModernizationSeam {
    pub id: String,
    pub state: ClaimState,
    pub location_node_id: String,
    pub location_name: String,
    pub seam_type: String,
    pub current_technology: String,
    pub leverage_score: u8,
    pub migration_risk_score: u8,
    pub isolation_score: u8,
    pub confidence_percent: u8,
    pub score_components: Vec<ScoreComponent>,
    pub evidence_ids: Vec<String>,
    pub recommended_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub factor: String,
    pub points: u8,
    pub reasoning: String,
}

/// A target-specific review item.  It deliberately reports observed evidence and
/// does not claim a migration outcome or a synthetic readiness percentage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityFinding {
    pub id: String,
    pub state: ClaimState,
    pub category: String,
    pub severity: String,
    pub summary: String,
    pub evidence_ids: Vec<String>,
    pub target_version: u16,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub schema_version: String,
    pub target_version: u16,
    pub findings: Vec<CompatibilityFinding>,
    pub limitations: Vec<String>,
}

pub const MIGRATION_PLAN_SCHEMA_VERSION: &str = "modernlink.migration-plan/v1alpha1";
pub const BOUNDARIES_SCHEMA_VERSION: &str = "modernlink.boundaries/v1alpha1";
pub const VERIFICATION_SCHEMA_VERSION: &str = "modernlink.verification/v1alpha1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    pub schema_version: String,
    pub checks: Vec<VerificationCheck>,
    pub limitations: Vec<String>,
}

impl VerificationReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .checks
            .sort_by(|left, right| left.id.cmp(&right.id));
        normalized.limitations.sort();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub id: String,
    pub status: String,
    pub summary: String,
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryReport {
    pub schema_version: String,
    pub boundaries: Vec<StaticBoundary>,
    pub limitations: Vec<String>,
}

impl BoundaryReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .boundaries
            .sort_by(|left, right| left.id.cmp(&right.id));
        normalized.limitations.sort();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticBoundary {
    pub id: String,
    pub state: ClaimState,
    pub kind: String,
    pub annotation: String,
    pub source_path: String,
    pub evidence_ids: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub schema_version: String,
    pub target_version: u16,
    pub tasks: Vec<MigrationPlanTask>,
}

impl MigrationPlan {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .tasks
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPlanTask {
    pub id: String,
    pub state: ClaimState,
    pub kind: String,
    pub summary: String,
    pub depends_on: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub approval_required: bool,
}

impl CompatibilityReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .findings
            .sort_by(|left, right| left.id.cmp(&right.id));
        normalized.limitations.sort();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

pub fn infer_structure(graph: &EvidenceGraph) -> Result<StructureReport, InferenceError> {
    graph.validate()?;
    let mut layers = Vec::new();
    let mut contexts = BTreeMap::<String, Vec<String>>::new();
    for node in &graph.nodes {
        if node.kind != "type" {
            continue;
        }
        if let Some(layer) = layer_for(&node.name) {
            layers.push(LayerInference {
                node_id: node.id.clone(),
                layer: layer.to_owned(),
                state: ClaimState::Inference,
                confidence_percent: 70,
                evidence_ids: node.evidence_ids.clone(),
                reasoning: "inferred from the declared type name".to_owned(),
            });
        }
        if let Some(context) = context_for(&node.name) {
            contexts
                .entry(context.to_owned())
                .or_default()
                .extend(node.evidence_ids.clone());
        }
    }
    let contexts = contexts
        .into_iter()
        .map(|(name, mut evidence_ids)| {
            evidence_ids.sort();
            evidence_ids.dedup();
            CandidateContext {
                name,
                state: ClaimState::Hypothesis,
                confidence_percent: 55,
                evidence_ids,
                reasoning:
                    "candidate context inferred from package cohesion; human confirmation required"
                        .to_owned(),
            }
        })
        .collect();
    Ok(StructureReport {
        schema_version: STRUCTURE_SCHEMA_VERSION.to_owned(),
        layers,
        contexts,
    })
}

pub fn infer_seams(graph: &EvidenceGraph) -> Result<SeamReport, InferenceError> {
    graph.validate()?;
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut seams = Vec::new();
    for edge in &graph.edges {
        if edge.kind != "imports" {
            continue;
        }
        let Some(target) = nodes.get(edge.target_id.as_str()) else {
            continue;
        };
        let Some(source) = nodes.get(edge.source_id.as_str()) else {
            continue;
        };
        if target.kind != "external-reference" {
            continue;
        }
        let technology = boundary_technology(&target.name).unwrap_or("external-api");
        let mut score_components = vec![ScoreComponent {
            factor: "external-boundary".to_owned(),
            points: 25,
            reasoning: "an import crosses from an owned node to an external reference".to_owned(),
        }];
        if technology != "external-api" {
            score_components.push(ScoreComponent {
                factor: boundary_factor(technology).to_owned(),
                points: boundary_points(technology),
                reasoning: boundary_reasoning(technology).to_owned(),
            });
        }
        let leverage_score = score_components.iter().map(|item| item.points).sum::<u8>();
        seams.push(ModernizationSeam {
            id: model::stable_id("modernization-seam", [edge.id.as_str()]),
            state: ClaimState::Inference,
            location_node_id: edge.source_id.clone(),
            location_name: source.name.clone(),
            seam_type: "outbound-import-dependency".to_owned(),
            current_technology: technology.to_owned(),
            leverage_score,
            migration_risk_score: boundary_risk(technology),
            isolation_score: boundary_isolation(technology),
            confidence_percent: if technology == "external-api" { 60 } else { 80 },
            score_components,
            evidence_ids: edge.evidence_ids.clone(),
            recommended_mode: boundary_mode(technology).to_owned(),
        });
    }
    Ok(SeamReport {
        schema_version: "modernlink.seams/v1alpha1".to_owned(),
        seams,
    })
}

/// Produces an evidence-linked target review from imports already present in the
/// graph. It is intentionally conservative: a finding asks for a review rather
/// than asserting that a dependency or source edit is sufficient.
pub fn assess_compatibility(
    graph: &EvidenceGraph,
    target_version: u16,
) -> Result<CompatibilityReport, InferenceError> {
    graph.validate()?;
    let mut findings = Vec::new();
    for node in &graph.nodes {
        if node.kind != "external-reference" {
            continue;
        }
        let Some((category, severity, summary, recommendation)) =
            compatibility_rule(&node.name, target_version)
        else {
            continue;
        };
        let mut evidence_ids = node.evidence_ids.clone();
        evidence_ids.extend(
            graph
                .edges
                .iter()
                .filter(|edge| edge.target_id == node.id && edge.kind == "imports")
                .flat_map(|edge| edge.evidence_ids.clone()),
        );
        evidence_ids.sort();
        evidence_ids.dedup();
        findings.push(CompatibilityFinding {
            id: model::stable_id(
                "compatibility-finding",
                [target_version.to_string().as_str(), node.id.as_str()],
            ),
            state: ClaimState::Inference,
            category: category.to_owned(),
            severity: severity.to_owned(),
            summary: summary.to_owned(),
            evidence_ids,
            target_version,
            recommendation: recommendation.to_owned(),
        });
    }
    Ok(CompatibilityReport {
        schema_version: COMPATIBILITY_SCHEMA_VERSION.to_owned(),
        target_version,
        findings,
        limitations: vec![
            "This report uses import evidence only; it does not yet inspect resolved dependency versions, bytecode, runtime configuration, or deployment behavior.".to_owned(),
            "A finding is an evidence-backed review request, not a claim that a migration will fail or that one change is sufficient.".to_owned(),
        ],
    })
}

/// Builds a minimal, review-oriented dependency DAG. It never chooses a target
/// technology; it makes the evidence already collected prerequisite work.
pub fn plan_migration(seams: &SeamReport, compatibility: &CompatibilityReport) -> MigrationPlan {
    let mut tasks = Vec::new();
    let mut compatibility_tasks = Vec::new();
    for finding in &compatibility.findings {
        let id = model::stable_id(
            "migration-plan-task",
            ["compatibility", finding.id.as_str()],
        );
        compatibility_tasks.push((id.clone(), finding));
        tasks.push(MigrationPlanTask {
            id,
            state: ClaimState::Inference,
            kind: "compatibility-review".to_owned(),
            summary: finding.summary.clone(),
            depends_on: Vec::new(),
            evidence_ids: finding.evidence_ids.clone(),
            approval_required: false,
        });
    }
    for seam in &seams.seams {
        let isolate_id = model::stable_id("migration-plan-task", ["isolate", seam.id.as_str()]);
        tasks.push(MigrationPlanTask {
            id: isolate_id.clone(),
            state: ClaimState::Inference,
            kind: "seam-isolation".to_owned(),
            summary: format!(
                "Isolate {} at {} before selecting a cutover action.",
                seam.current_technology, seam.location_name
            ),
            depends_on: Vec::new(),
            evidence_ids: seam.evidence_ids.clone(),
            approval_required: false,
        });
        let mut depends_on = vec![isolate_id];
        for (task_id, finding) in &compatibility_tasks {
            if finding
                .evidence_ids
                .iter()
                .any(|id| seam.evidence_ids.contains(id))
            {
                depends_on.push(task_id.clone());
            }
        }
        depends_on.sort();
        depends_on.dedup();
        tasks.push(MigrationPlanTask {
            id: model::stable_id("migration-plan-task", ["migrate", seam.id.as_str()]),
            state: ClaimState::Hypothesis,
            kind: "seam-migration".to_owned(),
            summary: format!(
                "Propose a {} migration for {} only after its listed prerequisites are reviewed.",
                seam.recommended_mode, seam.location_name
            ),
            depends_on,
            evidence_ids: seam.evidence_ids.clone(),
            approval_required: true,
        });
    }
    MigrationPlan {
        schema_version: MIGRATION_PLAN_SCHEMA_VERSION.to_owned(),
        target_version: compatibility.target_version,
        tasks,
    }
}

pub fn infer_boundaries(graph: &EvidenceGraph) -> Result<BoundaryReport, InferenceError> {
    graph.validate()?;
    let mut boundaries = Vec::new();
    for evidence in &graph.evidence {
        if evidence.kind != "annotation" {
            continue;
        }
        let Some((kind, reasoning)) = annotation_boundary(&evidence.value) else {
            continue;
        };
        boundaries.push(StaticBoundary {
            id: model::stable_id("static-boundary", [kind, evidence.id.as_str()]),
            state: ClaimState::Inference,
            kind: kind.to_owned(),
            annotation: evidence.value.clone(),
            source_path: evidence.source.path.clone(),
            evidence_ids: vec![evidence.id.clone()],
            reasoning: reasoning.to_owned(),
        });
    }
    Ok(BoundaryReport {
        schema_version: BOUNDARIES_SCHEMA_VERSION.to_owned(),
        boundaries,
        limitations: vec!["Annotation presence is source evidence only; this report does not establish runtime activation, entry ownership, transaction resources, or message destinations.".to_owned()],
    })
}

/// Checks only the static migration-plan contract. It deliberately has no authority
/// to assess runtime behavior or approve a lifecycle transition.
pub fn verify_plan(plan: &MigrationPlan) -> VerificationReport {
    let mut ids = std::collections::BTreeSet::new();
    let mut duplicate_ids = Vec::new();
    for task in &plan.tasks {
        if !ids.insert(task.id.clone()) {
            duplicate_ids.push(task.id.clone());
        }
    }
    let mut missing_dependencies = Vec::new();
    for task in &plan.tasks {
        for dependency in &task.depends_on {
            if !ids.contains(dependency) {
                missing_dependencies.push(task.id.clone());
            }
        }
    }
    let migration_without_gate = plan
        .tasks
        .iter()
        .filter(|task| task.kind == "seam-migration" && !task.approval_required)
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    VerificationReport {
        schema_version: VERIFICATION_SCHEMA_VERSION.to_owned(),
        checks: vec![
            VerificationCheck { id: "plan-unique-task-ids".to_owned(), status: if duplicate_ids.is_empty() { "OBSERVED" } else { "GAP" }.to_owned(), summary: "Every migration-plan task ID is unique.".to_owned(), task_ids: duplicate_ids },
            VerificationCheck { id: "plan-dependency-references".to_owned(), status: if missing_dependencies.is_empty() { "OBSERVED" } else { "GAP" }.to_owned(), summary: "Every listed prerequisite refers to a task in this migration plan.".to_owned(), task_ids: missing_dependencies },
            VerificationCheck { id: "migration-approval-gates".to_owned(), status: if migration_without_gate.is_empty() { "OBSERVED" } else { "GAP" }.to_owned(), summary: "Every seam-migration task declares human approval as required.".to_owned(), task_ids: migration_without_gate },
        ],
        limitations: vec![
            "This static report does not verify code behavior, tests, contracts, security, observability, performance, rollback, or production cutover safety.".to_owned(),
            "An OBSERVED plan check is not authorization to change lifecycle state or deploy a migration.".to_owned(),
        ],
    }
}

fn layer_for(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with("controller") || lower.ends_with("resource") {
        Some("presentation")
    } else if lower.ends_with("service") || lower.ends_with("usecase") {
        Some("service")
    } else if lower.ends_with("repository") || lower.ends_with("dao") {
        Some("persistence")
    } else if lower.ends_with("client") || lower.ends_with("gateway") || lower.ends_with("adapter")
    {
        Some("integration")
    } else {
        None
    }
}

fn context_for(name: &str) -> Option<&str> {
    let segments = name.split('.').collect::<Vec<_>>();
    (segments.len() >= 2).then(|| segments[segments.len() - 2])
}

fn annotation_boundary(annotation: &str) -> Option<(&'static str, &'static str)> {
    match annotation.rsplit('.').next().unwrap_or(annotation) {
        "Transactional" | "TransactionAttribute" => Some((
            "transaction",
            "inferred from a recognized transaction annotation",
        )),
        "MessageDriven" | "JmsListener" => Some((
            "messaging-consumer",
            "inferred from a recognized message-consumer annotation",
        )),
        "WebService" | "WebMethod" => Some(("soap", "inferred from a recognized SOAP annotation")),
        "Path" | "RequestMapping" => Some((
            "http",
            "inferred from a recognized HTTP endpoint annotation",
        )),
        "Scheduled" => Some((
            "batch",
            "inferred from a recognized scheduled-task annotation",
        )),
        _ => None,
    }
}

fn boundary_technology(name: &str) -> Option<&'static str> {
    if name.starts_with("weblogic.") {
        Some("weblogic")
    } else if name.starts_with("org.jboss.") {
        Some("jboss")
    } else if name.starts_with("com.ibm.websphere.") || name.starts_with("com.ibm.ws.") {
        Some("websphere")
    } else if name.starts_with("javax.jms.") || name.starts_with("jakarta.jms.") {
        Some("jms")
    } else if name.starts_with("javax.naming.") {
        Some("jndi")
    } else if name.starts_with("javax.sql.")
        || name.starts_with("java.sql.")
        || name.starts_with("javax.persistence.")
        || name.starts_with("org.hibernate.")
    {
        Some("database")
    } else if name.starts_with("javax.xml.ws.") || name.starts_with("jakarta.xml.ws.") {
        Some("soap")
    } else {
        None
    }
}

fn boundary_factor(technology: &str) -> &'static str {
    match technology {
        "weblogic" | "jboss" | "websphere" => "vendor-lock",
        "jms" => "messaging-boundary",
        "jndi" => "naming-boundary",
        "database" => "data-boundary",
        "soap" => "service-contract-boundary",
        _ => "external-boundary",
    }
}

fn boundary_points(technology: &str) -> u8 {
    match technology {
        "weblogic" | "jboss" | "websphere" => 45,
        "jms" | "database" => 35,
        "jndi" | "soap" => 30,
        _ => 0,
    }
}

fn boundary_reasoning(technology: &str) -> &'static str {
    match technology {
        "weblogic" | "jboss" | "websphere" => {
            "target name identifies application-server vendor coupling"
        }
        "jms" => "target name identifies a messaging API boundary",
        "jndi" => "target name identifies a naming lookup boundary",
        "database" => "target name identifies a persistence or JDBC boundary",
        "soap" => "target name identifies a SOAP client or endpoint boundary",
        _ => "target name identifies an external boundary",
    }
}

fn boundary_risk(technology: &str) -> u8 {
    match technology {
        "database" => 70,
        "jms" | "weblogic" | "jboss" | "websphere" => 60,
        "jndi" | "soap" => 50,
        _ => 40,
    }
}

fn boundary_isolation(technology: &str) -> u8 {
    match technology {
        "jms" | "soap" => 75,
        "jndi" => 70,
        "database" => 60,
        "weblogic" | "jboss" | "websphere" => 65,
        _ => 70,
    }
}

fn boundary_mode(technology: &str) -> &'static str {
    match technology {
        "jms" => "SHADOW -> MIRROR -> REDIRECT",
        "database" => "PASSTHROUGH -> SHADOW -> REDIRECT",
        "soap" => "PASSTHROUGH -> TRANSFORM -> REDIRECT",
        _ => "PASSTHROUGH -> SHADOW -> REDIRECT",
    }
}

fn compatibility_rule(
    name: &str,
    target_version: u16,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    if name.starts_with("sun.") || name.starts_with("com.sun.") {
        Some((
            "internal-jdk-api",
            "HIGH",
            "Import targets a non-standard JDK namespace.",
            "Establish the supported replacement and characterize behavior before selecting a target-runtime migration step.",
        ))
    } else if name.starts_with("weblogic.")
        || name.starts_with("org.jboss.")
        || name.starts_with("com.ibm.websphere.")
        || name.starts_with("com.ibm.ws.")
    {
        Some((
            "application-server-api",
            "HIGH",
            "Import targets an application-server-specific API.",
            "Isolate the vendor boundary and assess whether ModernLink can bridge it before changing the target runtime.",
        ))
    } else if target_version >= 11
        && (name.starts_with("javax.xml.bind.") || name.starts_with("javax.xml.ws."))
    {
        Some((
            "java-ee-api-review",
            "HIGH",
            "Import targets a legacy Java EE namespace that requires explicit target-runtime review.",
            "Inventory the deployment-provided API and dependency path, then choose a compatible migration boundary with characterization tests.",
        ))
    } else if name.starts_with("javax.") {
        Some((
            "java-ee-api-review",
            "MEDIUM",
            "Import targets a Java EE namespace whose runtime provider must be established for the selected target.",
            "Record the provider and deployment contract before changing runtime or namespace assumptions.",
        ))
    } else {
        None
    }
}

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GIT_HISTORY_SCHEMA_VERSION: &str = "modernlink.git-history/v1alpha1";

#[derive(Debug, Error)]
pub enum GitHistoryError {
    #[error("cannot open Git repository: {0}")]
    Repository(String),
    #[error("cannot resolve Git reference: {0}")]
    Reference(String),
    #[error("cannot traverse Git history: {0}")]
    Traversal(String),
    #[error("cannot access Git history cache: {0}")]
    Cache(String),
    #[error("cannot serialize Git history snapshot: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("cannot adapt Git history to evidence graph: {0}")]
    EvidenceGraph(#[from] model::GraphError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    pub repository_digest: String,
    pub object_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHistorySnapshot {
    pub schema_version: String,
    pub repository: RepositoryIdentity,
    #[serde(default)]
    pub selected_refs: Vec<SelectedRef>,
    #[serde(default)]
    pub completeness: Vec<Completeness>,
    #[serde(default)]
    pub commits: Vec<CommitFact>,
    #[serde(default)]
    pub path_changes: Vec<PathChange>,
    #[serde(default)]
    pub co_changes: Vec<CoChangeFact>,
    #[serde(default)]
    pub contributors: Vec<ContributorIdentity>,
    #[serde(default)]
    pub knowledge_signals: Vec<KnowledgeSignal>,
    #[serde(default)]
    pub metrics: Vec<HistoryMetric>,
}

impl GitHistorySnapshot {
    pub fn empty(repository: RepositoryIdentity) -> Self {
        Self {
            schema_version: GIT_HISTORY_SCHEMA_VERSION.to_owned(),
            repository,
            selected_refs: Vec::new(),
            completeness: Vec::new(),
            commits: Vec::new(),
            path_changes: Vec::new(),
            co_changes: Vec::new(),
            contributors: Vec::new(),
            knowledge_signals: Vec::new(),
            metrics: Vec::new(),
        }
    }

    pub fn canonical_json(&self) -> Result<String, GitHistoryError> {
        let mut normalized = self.clone();
        normalized.normalize();
        Ok(serde_json::to_string(&normalized)?)
    }

    pub fn to_evidence_graph(&self) -> Result<model::EvidenceGraph, GitHistoryError> {
        let mut graph = model::EvidenceGraph::new();
        let mut commit_evidence_ids = std::collections::BTreeMap::new();
        for commit in &self.commits {
            let evidence_id = model::stable_id("git-commit-evidence", [commit.object_id.as_str()]);
            graph.evidence.push(model::Evidence {
                id: evidence_id.clone(),
                kind: "git-commit".to_owned(),
                value: commit.object_id.clone(),
                source: model::SourceLocation {
                    path: ".git/history".to_owned(),
                    start_byte: 0,
                    end_byte: 0,
                },
            });
            graph.nodes.push(model::GraphNode {
                id: commit.object_id.clone(),
                kind: "commit".to_owned(),
                name: commit.object_id.clone(),
                evidence_ids: vec![evidence_id.clone()],
            });
            commit_evidence_ids.insert(commit.object_id.clone(), evidence_id);
        }
        for selected_ref in &self.selected_refs {
            let id = model::stable_id("git-ref-node", [selected_ref.name.as_str()]);
            graph.nodes.push(model::GraphNode {
                id: id.clone(),
                kind: "git-ref".to_owned(),
                name: selected_ref.name.clone(),
                evidence_ids: Vec::new(),
            });
            if let Some(evidence_id) = commit_evidence_ids.get(&selected_ref.target_id) {
                graph.edges.push(model::GraphEdge {
                    id: model::stable_id(
                        "git-ref-reaches-commit",
                        [id.as_str(), selected_ref.target_id.as_str()],
                    ),
                    kind: "REF_REACHES_COMMIT".to_owned(),
                    source_id: id,
                    target_id: selected_ref.target_id.clone(),
                    evidence_ids: vec![evidence_id.clone()],
                });
            }
        }
        for commit in &self.commits {
            let evidence_id = commit_evidence_ids
                .get(&commit.object_id)
                .expect("commit evidence is created with the commit")
                .clone();
            for parent_id in &commit.parent_ids {
                if commit_evidence_ids.contains_key(parent_id) {
                    graph.edges.push(model::GraphEdge {
                        id: model::stable_id(
                            "git-commit-parent",
                            [commit.object_id.as_str(), parent_id.as_str()],
                        ),
                        kind: "COMMIT_PARENT".to_owned(),
                        source_id: commit.object_id.clone(),
                        target_id: parent_id.clone(),
                        evidence_ids: vec![evidence_id.clone()],
                    });
                }
            }
        }
        let mut path_nodes = std::collections::BTreeMap::<String, (String, Vec<String>)>::new();
        for change in &self.path_changes {
            let evidence_id = model::stable_id(
                "git-path-change-evidence",
                [
                    change.commit_id.as_str(),
                    change.path.as_str(),
                    change.kind.as_str(),
                ],
            );
            graph.evidence.push(model::Evidence {
                id: evidence_id.clone(),
                kind: "git-path-change".to_owned(),
                value: format!("{}:{}", change.kind, change.path),
                source: model::SourceLocation {
                    path: change.path.clone(),
                    start_byte: 0,
                    end_byte: 0,
                },
            });
            let entry = path_nodes.entry(change.path.clone()).or_insert_with(|| {
                (
                    model::stable_id("git-path-node", [change.path.as_str()]),
                    Vec::new(),
                )
            });
            entry.1.push(evidence_id.clone());
            if commit_evidence_ids.contains_key(&change.commit_id) {
                graph.edges.push(model::GraphEdge {
                    id: model::stable_id(
                        "git-commit-touches-path",
                        [
                            change.commit_id.as_str(),
                            entry.0.as_str(),
                            evidence_id.as_str(),
                        ],
                    ),
                    kind: "COMMIT_TOUCHES_PATH".to_owned(),
                    source_id: change.commit_id.clone(),
                    target_id: entry.0.clone(),
                    evidence_ids: vec![evidence_id],
                });
            }
        }
        for (path, (id, evidence_ids)) in &path_nodes {
            graph.nodes.push(model::GraphNode {
                id: id.clone(),
                kind: "path".to_owned(),
                name: path.clone(),
                evidence_ids: evidence_ids.clone(),
            });
        }
        for co_change in &self.co_changes {
            let Some((left_id, _)) = path_nodes.get(&co_change.left_path) else {
                continue;
            };
            let Some((right_id, _)) = path_nodes.get(&co_change.right_path) else {
                continue;
            };
            let evidence_id = model::stable_id(
                "git-cochange-evidence",
                [co_change.left_path.as_str(), co_change.right_path.as_str()],
            );
            graph.evidence.push(model::Evidence {
                id: evidence_id.clone(),
                kind: "git-cochange".to_owned(),
                value: format!("{}|{}", co_change.left_path, co_change.right_path),
                source: model::SourceLocation {
                    path: ".git/history".to_owned(),
                    start_byte: 0,
                    end_byte: 0,
                },
            });
            graph.edges.push(model::GraphEdge {
                id: model::stable_id("git-path-cochange", [left_id.as_str(), right_id.as_str()]),
                kind: "PATH_CO_CHANGED_WITH_PATH".to_owned(),
                source_id: left_id.clone(),
                target_id: right_id.clone(),
                evidence_ids: vec![evidence_id],
            });
        }
        graph.validate()?;
        Ok(graph)
    }

    fn normalize(&mut self) {
        self.selected_refs
            .sort_by(|left, right| left.name.cmp(&right.name));
        self.completeness.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.subject.cmp(&right.subject))
        });
        self.commits
            .sort_by(|left, right| left.object_id.cmp(&right.object_id));
        self.path_changes.sort_by(|left, right| {
            left.commit_id
                .cmp(&right.commit_id)
                .then_with(|| left.path.cmp(&right.path))
        });
        self.co_changes.sort_by(|left, right| {
            left.left_path
                .cmp(&right.left_path)
                .then_with(|| left.right_path.cmp(&right.right_path))
        });
        self.contributors
            .sort_by(|left, right| left.comparison_key.cmp(&right.comparison_key));
        self.knowledge_signals.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.identity_key.cmp(&right.identity_key))
        });
        self.metrics.sort_by(|left, right| {
            left.subject
                .cmp(&right.subject)
                .then_with(|| left.name.cmp(&right.name))
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedRef {
    pub name: String,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Completeness {
    pub code: String,
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitFact {
    pub object_id: String,
    pub tree_id: String,
    pub parent_ids: Vec<String>,
    pub reachable_refs: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_identity_key: String,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_identity_key: String,
    pub author_time_seconds: i64,
    pub committer_time_seconds: i64,
    pub message_fingerprint: String,
    pub diff_parent_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathChange {
    pub commit_id: String,
    pub path: String,
    pub kind: String,
    pub additions: u64,
    pub deletions: u64,
    pub line_count_status: String,
    pub rename_detection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoChangeFact {
    pub left_path: String,
    pub right_path: String,
    pub commit_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributorIdentity {
    pub raw_name: String,
    pub raw_email: String,
    pub comparison_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeSignal {
    pub path: String,
    pub identity_key: String,
    pub changed_commit_count: u64,
    pub last_observed_time_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMetric {
    pub subject: String,
    pub name: String,
    pub numerator: u64,
    pub denominator: u64,
}

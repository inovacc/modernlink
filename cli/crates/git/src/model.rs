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
    #[error("cannot serialize Git history snapshot: {0}")]
    Serialization(#[from] serde_json::Error),
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
    pub author_identity_key: String,
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

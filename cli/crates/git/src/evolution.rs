use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::GitHistorySnapshot;

pub const EVOLUTION_SCHEMA_VERSION: &str = "modernlink.git-evolution/v1alpha1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionReport {
    pub schema_version: String,
    pub repository_digest: String,
    pub hotspots: Vec<PathHotspot>,
    pub co_changes: Vec<EvolutionXray>,
    pub contributors: Vec<ContributorContinuity>,
    pub limitations: Vec<String>,
}

impl EvolutionReport {
    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        let mut normalized = self.clone();
        normalized.normalize();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }

    fn normalize(&mut self) {
        self.hotspots.sort_by(|left, right| {
            right
                .changed_commit_count
                .cmp(&left.changed_commit_count)
                .then_with(|| left.path.cmp(&right.path))
        });
        self.co_changes.sort_by(|left, right| {
            right
                .changed_commit_count
                .cmp(&left.changed_commit_count)
                .then_with(|| left.left_path.cmp(&right.left_path))
                .then_with(|| left.right_path.cmp(&right.right_path))
        });
        self.contributors
            .sort_by(|left, right| left.identity_key.cmp(&right.identity_key));
        self.limitations.sort();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathHotspot {
    pub path: String,
    pub changed_commit_count: u64,
    pub additions: u64,
    pub deletions: u64,
    pub line_count_status: String,
    pub contributor_identity_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionXray {
    pub left_path: String,
    pub right_path: String,
    pub changed_commit_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributorContinuity {
    pub identity_key: String,
    pub authored_commit_count: u64,
    pub touched_path_count: u64,
    pub last_observed_time_seconds: i64,
}

pub fn xray_history(snapshot: &GitHistorySnapshot) -> EvolutionReport {
    let commits = snapshot
        .commits
        .iter()
        .map(|commit| (commit.object_id.as_str(), commit))
        .collect::<BTreeMap<_, _>>();
    let mut hotspots =
        BTreeMap::<String, (BTreeSet<String>, u64, u64, bool, BTreeSet<String>)>::new();
    let mut contributor_paths = BTreeMap::<String, BTreeSet<String>>::new();
    let mut contributor_commits = BTreeMap::<String, BTreeSet<String>>::new();
    let mut contributor_last_seen = BTreeMap::<String, i64>::new();
    for change in &snapshot.path_changes {
        let entry = hotspots.entry(change.path.clone()).or_default();
        entry.0.insert(change.commit_id.clone());
        entry.1 += change.additions;
        entry.2 += change.deletions;
        entry.3 |= change.line_count_status != "measured";
        if let Some(commit) = commits.get(change.commit_id.as_str()) {
            entry.4.insert(commit.author_identity_key.clone());
            contributor_paths
                .entry(commit.author_identity_key.clone())
                .or_default()
                .insert(change.path.clone());
            contributor_commits
                .entry(commit.author_identity_key.clone())
                .or_default()
                .insert(change.commit_id.clone());
            contributor_last_seen
                .entry(commit.author_identity_key.clone())
                .and_modify(|time| *time = (*time).max(commit.author_time_seconds))
                .or_insert(commit.author_time_seconds);
        }
    }
    let hotspots = hotspots
        .into_iter()
        .map(
            |(path, (commits, additions, deletions, incomplete, contributors))| PathHotspot {
                path,
                changed_commit_count: commits.len() as u64,
                additions,
                deletions,
                line_count_status: if incomplete {
                    "partially-unavailable"
                } else {
                    "measured"
                }
                .to_owned(),
                contributor_identity_keys: contributors.into_iter().collect(),
            },
        )
        .collect();
    let co_changes = snapshot
        .co_changes
        .iter()
        .map(|item| EvolutionXray {
            left_path: item.left_path.clone(),
            right_path: item.right_path.clone(),
            changed_commit_count: item.commit_ids.len() as u64,
        })
        .collect();
    let contributors = contributor_commits
        .into_iter()
        .map(|(identity_key, commits)| ContributorContinuity {
            touched_path_count: contributor_paths
                .get(&identity_key)
                .map_or(0, |paths| paths.len() as u64),
            last_observed_time_seconds: contributor_last_seen
                .get(&identity_key)
                .copied()
                .unwrap_or_default(),
            identity_key,
            authored_commit_count: commits.len() as u64,
        })
        .collect();
    let mut report = EvolutionReport {
        schema_version: EVOLUTION_SCHEMA_VERSION.to_owned(),
        repository_digest: snapshot.repository.repository_digest.clone(),
        hotspots,
        co_changes,
        contributors,
        limitations: vec![
            "Contributor fields are continuity signals, not productivity or quality rankings.".to_owned(),
            "Line counts and co-change relationships inherit the history collector's completeness limits.".to_owned(),
        ],
    };
    report.normalize();
    report
}

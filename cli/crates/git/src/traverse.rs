use std::collections::{BTreeMap, BTreeSet};

use gix::{bstr::ByteSlice, object::tree::diff::ChangeDetached, prelude::TreeDiffChangeExt};
use sha2::{Digest, Sha256};

use crate::{
    CacheKey, CoChangeFact, CommitFact, Completeness, ContributorIdentity, GitHistoryError,
    GitHistorySnapshot, HistoryMetric, KnowledgeSignal, PathChange, RefScope, RepositoryIdentity,
    SelectedRef, load_cached, open_repository, resolve_refs, store_cached,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailmapMode {
    Off,
    Repository,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryOptions {
    pub ref_scope: RefScope,
    pub max_commits: usize,
    pub max_cochange_paths: usize,
    pub mailmap: MailmapMode,
}

impl HistoryOptions {
    pub fn all() -> Self {
        Self {
            ref_scope: RefScope::All,
            max_commits: usize::MAX,
            max_cochange_paths: 256,
            mailmap: MailmapMode::Off,
        }
    }
}

pub fn collect_history(
    path: &std::path::Path,
    options: &HistoryOptions,
) -> Result<GitHistorySnapshot, GitHistoryError> {
    let repository = open_repository(path)?;
    let selected_refs = resolve_refs(&repository, options.ref_scope.clone())?;
    collect_history_from_repository(&repository, selected_refs, options)
}

pub fn collect_history_cached(
    path: &std::path::Path,
    options: &HistoryOptions,
) -> Result<GitHistorySnapshot, GitHistoryError> {
    let repository = open_repository(path)?;
    let selected_refs = resolve_refs(&repository, options.ref_scope.clone())?;
    let cache_key = CacheKey::from_history_inputs(&repository, options, &selected_refs);
    let cache_root = repository.workdir().ok_or_else(|| {
        GitHistoryError::Cache(
            "cannot place a project-local cache for a bare repository".to_owned(),
        )
    })?;
    if let Some(snapshot) = load_cached(cache_root, &cache_key)? {
        return Ok(snapshot);
    }
    let snapshot = collect_history_from_repository(&repository, selected_refs, options)?;
    store_cached(cache_root, &cache_key, &snapshot)?;
    Ok(snapshot)
}

fn collect_history_from_repository(
    repository: &gix::Repository,
    selected_refs: Vec<SelectedRef>,
    options: &HistoryOptions,
) -> Result<GitHistorySnapshot, GitHistoryError> {
    let identity = repository_identity(&repository, &selected_refs);
    let mut facts = BTreeMap::<String, CollectedCommit>::new();
    let mut limit_reached = false;

    'references: for selected_ref in &selected_refs {
        let target_id = selected_ref
            .target_id
            .parse::<gix::ObjectId>()
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
        let walk = repository
            .rev_walk([target_id])
            .all()
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;

        for item in walk {
            let info = item.map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
            let object_id = info.id.to_string();
            if let Some(collected) = facts.get_mut(&object_id) {
                collected.reachable_refs.insert(selected_ref.name.clone());
                continue;
            }
            if facts.len() == options.max_commits {
                limit_reached = true;
                break 'references;
            }

            let commit = repository
                .find_commit(info.id)
                .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
            let mut fact = commit_fact(&commit)?;
            fact.reachable_refs.push(selected_ref.name.clone());
            facts.insert(
                object_id,
                CollectedCommit {
                    fact,
                    reachable_refs: BTreeSet::from([selected_ref.name.clone()]),
                },
            );
        }
    }

    let mut snapshot = GitHistorySnapshot::empty(identity);
    snapshot.selected_refs = selected_refs;
    if limit_reached {
        snapshot.completeness.push(Completeness {
            code: "commit-limit-reached".to_owned(),
            subject: "history".to_owned(),
            detail: format!("max_commits={}", options.max_commits),
        });
    }
    for collected in facts.into_values() {
        let mut fact = collected.fact;
        fact.reachable_refs = collected.reachable_refs.into_iter().collect();
        snapshot.commits.push(fact);
    }
    snapshot.contributors = collect_contributors(&snapshot.commits);
    if options.mailmap == MailmapMode::Repository {
        snapshot.completeness.push(Completeness {
            code: "mailmap-not-applied".to_owned(),
            subject: "identity-normalization".to_owned(),
            detail: "repository mailmap support is not implemented in this collector version"
                .to_owned(),
        });
    }
    snapshot.path_changes = collect_path_changes(repository, &snapshot.commits)?;
    let (co_changes, co_change_completeness) =
        collect_co_changes(&snapshot.path_changes, options.max_cochange_paths);
    snapshot.co_changes = co_changes;
    snapshot.completeness.extend(co_change_completeness);
    snapshot.knowledge_signals =
        collect_knowledge_signals(&snapshot.commits, &snapshot.path_changes);
    snapshot.metrics = collect_metrics(&snapshot.commits, &snapshot.path_changes);
    Ok(snapshot)
}

struct CollectedCommit {
    fact: CommitFact,
    reachable_refs: BTreeSet<String>,
}

fn repository_identity(
    repository: &gix::Repository,
    selected_refs: &[SelectedRef],
) -> RepositoryIdentity {
    let mut digest = Sha256::new();
    digest.update(format!("{:?}\n", repository.object_hash()));
    for selected_ref in selected_refs {
        digest.update(selected_ref.name.as_bytes());
        digest.update([0]);
        digest.update(selected_ref.target_id.as_bytes());
        digest.update([b'\n']);
    }
    RepositoryIdentity {
        repository_digest: format!("repository:sha256:{}", hex::encode(digest.finalize())),
        object_format: format!("{:?}", repository.object_hash()).to_lowercase(),
    }
}

fn commit_fact(commit: &gix::Commit<'_>) -> Result<CommitFact, GitHistoryError> {
    let author = commit
        .author()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
    let committer = commit
        .committer()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
    let author_time_seconds = author
        .time()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?
        .seconds;
    let committer_time_seconds = committer
        .time()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?
        .seconds;
    let tree_id = commit
        .tree_id()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?
        .detach()
        .to_string();
    let message = commit
        .message_raw()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
    let message_bytes: &[u8] = message.as_ref();
    let author_name = identity_component(author.name.as_ref());
    let author_email = identity_component(author.email.as_ref());
    let committer_name = identity_component(committer.name.as_ref());
    let committer_email = identity_component(committer.email.as_ref());

    Ok(CommitFact {
        object_id: commit.id().detach().to_string(),
        tree_id,
        parent_ids: commit
            .parent_ids()
            .map(|parent| parent.detach().to_string())
            .collect(),
        reachable_refs: Vec::new(),
        author_identity_key: identity_key(&author_name, &author_email),
        author_name,
        author_email,
        committer_identity_key: identity_key(&committer_name, &committer_email),
        committer_name,
        committer_email,
        author_time_seconds,
        committer_time_seconds,
        message_fingerprint: format!("sha256:{}", hex::encode(Sha256::digest(message_bytes))),
        diff_parent_policy: "first-parent".to_owned(),
    })
}

fn collect_contributors(commits: &[CommitFact]) -> Vec<ContributorIdentity> {
    let mut identities = BTreeMap::<String, ContributorIdentity>::new();
    for commit in commits {
        for (key, raw_name, raw_email) in [
            (
                &commit.author_identity_key,
                &commit.author_name,
                &commit.author_email,
            ),
            (
                &commit.committer_identity_key,
                &commit.committer_name,
                &commit.committer_email,
            ),
        ] {
            identities
                .entry(key.clone())
                .or_insert_with(|| ContributorIdentity {
                    raw_name: raw_name.clone(),
                    raw_email: raw_email.clone(),
                    comparison_key: key.clone(),
                });
        }
    }
    identities.into_values().collect()
}

fn collect_path_changes(
    repository: &gix::Repository,
    commits: &[CommitFact],
) -> Result<Vec<PathChange>, GitHistoryError> {
    let mut path_changes = Vec::new();
    for fact in commits {
        let current_tree_id = fact
            .tree_id
            .parse::<gix::ObjectId>()
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
        let current_tree = repository
            .find_tree(current_tree_id)
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
        let parent_tree = fact
            .parent_ids
            .first()
            .map(|parent_id| {
                let parent_id = parent_id
                    .parse::<gix::ObjectId>()
                    .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
                let parent = repository
                    .find_commit(parent_id)
                    .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
                let parent_tree_id = parent
                    .tree_id()
                    .map_err(|error| GitHistoryError::Traversal(error.to_string()))?
                    .detach();
                repository
                    .find_tree(parent_tree_id)
                    .map_err(|error| GitHistoryError::Traversal(error.to_string()))
            })
            .transpose()?;
        let mut options = gix::diff::Options::default();
        options.track_path().track_rewrites(None);
        let changes = repository
            .diff_tree_to_tree(parent_tree.as_ref(), &current_tree, options)
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
        let mut resource_cache = repository
            .diff_resource_cache(gix::diff::blob::pipeline::Mode::ToGit, Default::default())
            .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;

        for change in changes {
            let (kind, path) = change_kind_and_path(&change);
            let (additions, deletions, line_count_status) =
                line_counts(&change, repository, &mut resource_cache)?;
            path_changes.push(PathChange {
                commit_id: fact.object_id.clone(),
                path,
                kind: kind.to_owned(),
                additions,
                deletions,
                line_count_status,
                rename_detection: "disabled".to_owned(),
            });
        }
    }
    path_changes.sort_by(|left, right| {
        left.commit_id
            .cmp(&right.commit_id)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(path_changes)
}

fn change_kind_and_path(change: &ChangeDetached) -> (&'static str, String) {
    match change {
        ChangeDetached::Addition { location, .. } => {
            ("addition", location.to_str_lossy().into_owned())
        }
        ChangeDetached::Deletion { location, .. } => {
            ("deletion", location.to_str_lossy().into_owned())
        }
        ChangeDetached::Modification { location, .. } => {
            ("modification", location.to_str_lossy().into_owned())
        }
        ChangeDetached::Rewrite { location, .. } => {
            ("rewrite", location.to_str_lossy().into_owned())
        }
    }
}

fn line_counts(
    change: &ChangeDetached,
    repository: &gix::Repository,
    resource_cache: &mut gix::diff::blob::Platform,
) -> Result<(u64, u64, String), GitHistoryError> {
    let attached = change.attach(repository, repository);
    let counts = attached
        .diff(resource_cache)
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?
        .line_counts()
        .map_err(|error| GitHistoryError::Traversal(error.to_string()))?;
    Ok(match counts {
        Some(counts) => (
            u64::from(counts.insertions),
            u64::from(counts.removals),
            "measured".to_owned(),
        ),
        None => (0, 0, "unavailable".to_owned()),
    })
}

fn collect_co_changes(
    path_changes: &[PathChange],
    max_cochange_paths: usize,
) -> (Vec<CoChangeFact>, Vec<Completeness>) {
    let mut changed_paths_by_commit = BTreeMap::<String, BTreeSet<String>>::new();
    for change in path_changes {
        changed_paths_by_commit
            .entry(change.commit_id.clone())
            .or_default()
            .insert(change.path.clone());
    }
    let mut pairs = BTreeMap::<(String, String), BTreeSet<String>>::new();
    let mut completeness = Vec::new();
    for (commit_id, paths) in changed_paths_by_commit {
        if paths.len() > max_cochange_paths {
            completeness.push(Completeness {
                code: "cochange-path-cap-reached".to_owned(),
                subject: commit_id,
                detail: format!("max_cochange_paths={max_cochange_paths}"),
            });
            continue;
        }
        let paths = paths.into_iter().collect::<Vec<_>>();
        for (index, left_path) in paths.iter().enumerate() {
            for right_path in paths.iter().skip(index + 1) {
                pairs
                    .entry((left_path.clone(), right_path.clone()))
                    .or_default()
                    .insert(commit_id.clone());
            }
        }
    }
    let co_changes = pairs
        .into_iter()
        .map(|((left_path, right_path), commit_ids)| CoChangeFact {
            left_path,
            right_path,
            commit_ids: commit_ids.into_iter().collect(),
        })
        .collect();
    (co_changes, completeness)
}

fn collect_knowledge_signals(
    commits: &[CommitFact],
    path_changes: &[PathChange],
) -> Vec<KnowledgeSignal> {
    let commits_by_id = commits
        .iter()
        .map(|commit| (commit.object_id.as_str(), commit))
        .collect::<BTreeMap<_, _>>();
    let mut signals = BTreeMap::<(String, String), (BTreeSet<String>, i64)>::new();
    for path_change in path_changes {
        let Some(commit) = commits_by_id.get(path_change.commit_id.as_str()) else {
            continue;
        };
        let entry = signals
            .entry((path_change.path.clone(), commit.author_identity_key.clone()))
            .or_insert_with(|| (BTreeSet::new(), commit.author_time_seconds));
        entry.0.insert(path_change.commit_id.clone());
        entry.1 = entry.1.max(commit.author_time_seconds);
    }
    signals
        .into_iter()
        .map(
            |((path, identity_key), (commit_ids, last_observed_time_seconds))| KnowledgeSignal {
                path,
                identity_key,
                changed_commit_count: commit_ids.len() as u64,
                last_observed_time_seconds,
            },
        )
        .collect()
}

fn collect_metrics(commits: &[CommitFact], path_changes: &[PathChange]) -> Vec<HistoryMetric> {
    let mut commit_counts = BTreeMap::<String, u64>::new();
    for commit in commits {
        *commit_counts
            .entry(commit.author_identity_key.clone())
            .or_default() += 1;
    }
    let mut changed_commit_counts = BTreeMap::<String, BTreeSet<String>>::new();
    for path_change in path_changes {
        changed_commit_counts
            .entry(path_change.path.clone())
            .or_default()
            .insert(path_change.commit_id.clone());
    }
    let commit_total = commits.len() as u64;
    let mut metrics = commit_counts
        .into_iter()
        .map(|(subject, numerator)| HistoryMetric {
            subject,
            name: "commit-count".to_owned(),
            numerator,
            denominator: commit_total,
        })
        .collect::<Vec<_>>();
    metrics.extend(
        changed_commit_counts
            .into_iter()
            .map(|(subject, commit_ids)| HistoryMetric {
                subject,
                name: "changed-commit-count".to_owned(),
                numerator: commit_ids.len() as u64,
                denominator: commit_total,
            }),
    );
    metrics
}

fn identity_component(value: &[u8]) -> String {
    String::from_utf8_lossy(value).into_owned()
}

fn identity_key(name: &str, email: &str) -> String {
    format!("{name}\u{0}{}", email.to_lowercase())
}

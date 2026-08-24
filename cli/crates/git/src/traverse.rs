use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::{
    CommitFact, Completeness, ContributorIdentity, GitHistoryError, GitHistorySnapshot, RefScope,
    RepositoryIdentity, SelectedRef, open_repository, resolve_refs,
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

    Ok(CommitFact {
        object_id: commit.id().detach().to_string(),
        tree_id,
        parent_ids: commit
            .parent_ids()
            .map(|parent| parent.detach().to_string())
            .collect(),
        reachable_refs: Vec::new(),
        author_identity_key: identity_key(author.name.as_ref(), author.email.as_ref()),
        committer_identity_key: identity_key(committer.name.as_ref(), committer.email.as_ref()),
        author_time_seconds,
        committer_time_seconds,
        message_fingerprint: format!("sha256:{}", hex::encode(Sha256::digest(message_bytes))),
        diff_parent_policy: "first-parent".to_owned(),
    })
}

fn collect_contributors(commits: &[CommitFact]) -> Vec<ContributorIdentity> {
    let mut identities = BTreeMap::<String, ContributorIdentity>::new();
    for commit in commits {
        for key in [&commit.author_identity_key, &commit.committer_identity_key] {
            let (raw_name, raw_email) = key.split_once('\u{0}').unwrap_or((key, ""));
            identities
                .entry(key.clone())
                .or_insert_with(|| ContributorIdentity {
                    raw_name: raw_name.to_owned(),
                    raw_email: raw_email.to_owned(),
                    comparison_key: key.clone(),
                });
        }
    }
    identities.into_values().collect()
}

fn identity_key(name: &[u8], email: &[u8]) -> String {
    let raw_name = String::from_utf8_lossy(name).trim().to_owned();
    let raw_email = String::from_utf8_lossy(email).trim().to_owned();
    format!("{raw_name}\u{0}{raw_email}")
}

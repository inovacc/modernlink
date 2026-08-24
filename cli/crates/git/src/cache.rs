use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

use crate::{GitHistoryError, GitHistorySnapshot, HistoryOptions, SelectedRef};

const CACHE_FORMAT_VERSION: &str = "modernlink.git-history-cache/v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    pub fn from_parts<I, S, T>(
        object_format: &str,
        selected_refs: I,
        max_commits: usize,
        max_cochange_paths: usize,
    ) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let mut selected_refs = selected_refs
            .into_iter()
            .map(|(name, target)| (name.as_ref().to_owned(), target.as_ref().to_owned()))
            .collect::<Vec<_>>();
        selected_refs.sort();
        let mut digest = Sha256::new();
        digest.update(CACHE_FORMAT_VERSION.as_bytes());
        digest.update([0]);
        digest.update(object_format.as_bytes());
        digest.update([0]);
        digest.update(max_commits.to_le_bytes());
        digest.update(max_cochange_paths.to_le_bytes());
        for (name, target) in selected_refs {
            digest.update(name.as_bytes());
            digest.update([0]);
            digest.update(target.as_bytes());
            digest.update([0]);
        }
        Self(hex::encode(digest.finalize()))
    }

    pub fn from_history_inputs(
        repository: &gix::Repository,
        options: &HistoryOptions,
        selected_refs: &[SelectedRef],
    ) -> Self {
        let mut key = Self::from_parts(
            &format!("{:?}", repository.object_hash()).to_lowercase(),
            selected_refs
                .iter()
                .map(|selected_ref| (selected_ref.name.as_str(), selected_ref.target_id.as_str())),
            options.max_commits,
            options.max_cochange_paths,
        );
        let mut digest = Sha256::new();
        digest.update(CACHE_FORMAT_VERSION.as_bytes());
        digest.update([0]);
        digest.update(key.0.as_bytes());
        digest.update([0]);
        digest.update(format!("{:?}", options.ref_scope).as_bytes());
        digest.update([0]);
        digest.update(format!("{:?}", options.mailmap).as_bytes());
        key.0 = hex::encode(digest.finalize());
        key
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn load_cached(
    repository_root: &Path,
    key: &CacheKey,
) -> Result<Option<GitHistorySnapshot>, GitHistoryError> {
    let path = cache_file(repository_root, key, false)?;
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| GitHistoryError::Cache(error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(GitHistoryError::Serialization)
}

pub fn store_cached(
    repository_root: &Path,
    key: &CacheKey,
    snapshot: &GitHistorySnapshot,
) -> Result<(), GitHistoryError> {
    let path = cache_file(repository_root, key, true)?;
    if path.exists() {
        return Ok(());
    }
    let json = snapshot.canonical_json()?;
    let parent = path
        .parent()
        .ok_or_else(|| GitHistoryError::Cache("cache file has no parent directory".to_owned()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| GitHistoryError::Cache(error.to_string()))?
        .as_nanos();
    let temporary = parent.join(format!(".{}.{}.tmp", key.as_str(), nonce));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| GitHistoryError::Cache(error.to_string()))?;
        file.write_all(json.as_bytes())
            .map_err(|error| GitHistoryError::Cache(error.to_string()))?;
        file.sync_all()
            .map_err(|error| GitHistoryError::Cache(error.to_string()))?;
        fs::rename(&temporary, &path).map_err(|error| GitHistoryError::Cache(error.to_string()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    if result.is_err() && path.exists() {
        return Ok(());
    }
    result
}

fn cache_file(
    repository_root: &Path,
    key: &CacheKey,
    create: bool,
) -> Result<PathBuf, GitHistoryError> {
    let root = fs::canonicalize(repository_root).map_err(|error| {
        GitHistoryError::Cache(format!("cannot resolve repository root: {error}"))
    })?;
    if !root.is_dir() {
        return Err(GitHistoryError::Cache(format!(
            "repository root is not a directory: {}",
            root.display()
        )));
    }
    let cache_dir = root.join(".modernlink/cache/git");
    if create {
        fs::create_dir_all(&cache_dir)
            .map_err(|error| GitHistoryError::Cache(error.to_string()))?;
    }
    if !cache_dir.exists() {
        return Ok(cache_dir.join(format!("{}.json", key.as_str())));
    }
    let cache_dir =
        fs::canonicalize(&cache_dir).map_err(|error| GitHistoryError::Cache(error.to_string()))?;
    if !cache_dir.starts_with(&root) {
        return Err(GitHistoryError::Cache(format!(
            "cache path escapes repository root: {}",
            cache_dir.display()
        )));
    }
    Ok(cache_dir.join(format!("{}.json", key.as_str())))
}

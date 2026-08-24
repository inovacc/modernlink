use git::{CacheKey, GitHistorySnapshot, RepositoryIdentity, load_cached, store_cached};
use tempfile::tempdir;

#[test]
fn cache_key_changes_when_selected_ref_target_changes() {
    let first = CacheKey::from_parts("sha1", [("refs/heads/main", "a".repeat(40))], 10, 100);
    let second = CacheKey::from_parts("sha1", [("refs/heads/main", "b".repeat(40))], 10, 100);

    assert_ne!(first, second);
}

#[test]
fn local_cache_round_trips_canonical_history_without_leaving_the_repository() {
    let repository = tempdir().expect("temporary repository root");
    let key = CacheKey::from_parts("sha1", [("refs/heads/main", "c".repeat(40))], 10, 100);
    let snapshot = GitHistorySnapshot::empty(RepositoryIdentity {
        repository_digest: "repository:sha256:fixture".to_owned(),
        object_format: "sha1".to_owned(),
    });

    store_cached(repository.path(), &key, &snapshot).expect("store local history cache");
    let cached = load_cached(repository.path(), &key)
        .expect("load local history cache")
        .expect("cache entry");

    assert_eq!(cached, snapshot);
    assert!(repository.path().join(".modernlink/cache/git").is_dir());
}

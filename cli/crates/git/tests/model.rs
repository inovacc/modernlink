use git::{GitHistorySnapshot, RepositoryIdentity};

#[test]
fn canonical_json_is_stable_and_round_trips() {
    let snapshot = GitHistorySnapshot::empty(RepositoryIdentity {
        repository_digest: "repository:sha256:0123456789abcdef".to_owned(),
        object_format: "sha1".to_owned(),
    });

    let first = snapshot.canonical_json().expect("canonical JSON");
    let second = snapshot.canonical_json().expect("canonical JSON");

    assert_eq!(first, second);
    assert!(first.contains("modernlink.git-history/v1alpha1"));
    assert_eq!(
        serde_json::from_str::<GitHistorySnapshot>(&first).expect("snapshot round trip"),
        snapshot
    );
}

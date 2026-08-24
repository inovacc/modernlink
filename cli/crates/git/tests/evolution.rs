use git::{CommitFact, GitHistorySnapshot, PathChange, RepositoryIdentity, xray_history};

#[test]
fn xray_reports_change_hotspots_and_continuity_without_contributor_ranking() {
    let mut history = GitHistorySnapshot::empty(RepositoryIdentity {
        repository_digest: "repository:fixture".to_owned(),
        object_format: "sha1".to_owned(),
    });
    history.commits = vec![
        commit("c1", "alice\0alice@example.com", 100),
        commit("c2", "bob\0bob@example.com", 200),
    ];
    history.path_changes = vec![
        change("c1", "src/Payment.java", 10, 2),
        change("c2", "src/Payment.java", 4, 1),
        change("c2", "src/Other.java", 1, 0),
    ];
    let report = xray_history(&history);
    assert_eq!(report.hotspots[0].path, "src/Payment.java");
    assert_eq!(report.hotspots[0].changed_commit_count, 2);
    assert_eq!(report.contributors.len(), 2);
    assert!(
        report
            .limitations
            .iter()
            .any(|item| item.contains("not productivity"))
    );
}

fn commit(id: &str, identity: &str, time: i64) -> CommitFact {
    CommitFact {
        object_id: id.to_owned(),
        tree_id: "tree".to_owned(),
        parent_ids: Vec::new(),
        reachable_refs: Vec::new(),
        author_name: identity.split('\0').next().unwrap().to_owned(),
        author_email: identity.split('\0').nth(1).unwrap().to_owned(),
        author_identity_key: identity.to_owned(),
        committer_name: "committer".to_owned(),
        committer_email: "committer@example.com".to_owned(),
        committer_identity_key: "committer\0committer@example.com".to_owned(),
        author_time_seconds: time,
        committer_time_seconds: time,
        message_fingerprint: "sha256:message".to_owned(),
        diff_parent_policy: "first-parent".to_owned(),
    }
}

fn change(commit_id: &str, path: &str, additions: u64, deletions: u64) -> PathChange {
    PathChange {
        commit_id: commit_id.to_owned(),
        path: path.to_owned(),
        kind: "modification".to_owned(),
        additions,
        deletions,
        line_count_status: "measured".to_owned(),
        rename_detection: "disabled".to_owned(),
    }
}

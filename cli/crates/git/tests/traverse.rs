use git::{HistoryOptions, collect_history, collect_history_cached};
use gix::bstr::ByteSlice;
use tempfile::tempdir;

#[test]
fn traversal_keeps_raw_identity_and_records_commit_limit() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let tree = repository.empty_tree().id().detach();
    let alice = signature(b"Alice Example", b"alice@example.test", "1 +0000");
    let abbreviated = signature(b"A. Example", b"alice@example.test", "2 +0000");

    let first = repository
        .commit_as(
            alice,
            alice,
            "refs/heads/main",
            "first\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create first commit")
        .detach();
    let second = repository
        .commit_as(
            abbreviated,
            abbreviated,
            "refs/heads/main",
            "second\n",
            tree,
            std::iter::once(first),
        )
        .expect("create second commit")
        .detach();
    repository
        .commit_as(
            alice,
            alice,
            "refs/heads/main",
            "third\n",
            tree,
            std::iter::once(second),
        )
        .expect("create third commit");

    let report = collect_history(
        directory.path(),
        &HistoryOptions {
            max_commits: 2,
            ..HistoryOptions::all()
        },
    )
    .expect("collect history");

    assert_eq!(report.commits.len(), 2);
    assert!(
        report
            .completeness
            .iter()
            .any(|item| item.code == "commit-limit-reached")
    );
    assert!(
        report
            .contributors
            .iter()
            .any(|identity| identity.raw_name == "Alice Example")
    );
    assert!(
        report
            .contributors
            .iter()
            .any(|identity| identity.raw_name == "A. Example")
    );
    assert!(report.commits.iter().all(|commit| {
        commit.diff_parent_policy == "first-parent" && !commit.reachable_refs.is_empty()
    }));
    assert!(report.commits.iter().any(|commit| {
        commit.author_name == "Alice Example" && commit.author_email == "alice@example.test"
    }));
    let graph = report
        .to_evidence_graph()
        .expect("shared Git evidence graph");
    assert!(graph.validate().is_ok());
    assert!(graph.edges.iter().any(|edge| edge.kind == "COMMIT_PARENT"));
}

#[test]
fn cached_traversal_writes_repository_local_evidence() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let tree = repository.empty_tree().id().detach();
    let author = signature(b"Cache Historian", b"cache@example.test", "1 +0000");
    repository
        .commit_as(
            author,
            author,
            "refs/heads/main",
            "first\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit");

    let first = collect_history_cached(directory.path(), &HistoryOptions::all())
        .expect("collect cached history");
    let second = collect_history_cached(directory.path(), &HistoryOptions::all())
        .expect("reuse cached history");

    assert_eq!(
        first.canonical_json().unwrap(),
        second.canonical_json().unwrap()
    );
    assert!(directory.path().join(".modernlink/cache/git").is_dir());
}

#[test]
fn requested_mailmap_is_reported_when_not_applied() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let tree = repository.empty_tree().id().detach();
    let author = signature(b"Mailmap Historian", b"mailmap@example.test", "1 +0000");
    repository
        .commit_as(
            author,
            author,
            "refs/heads/main",
            "first\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit");

    let report = collect_history(
        directory.path(),
        &HistoryOptions {
            mailmap: git::MailmapMode::Repository,
            ..HistoryOptions::all()
        },
    )
    .expect("collect history");

    assert!(
        report
            .completeness
            .iter()
            .any(|item| item.code == "mailmap-not-applied")
    );
}

fn signature(
    name: &'static [u8],
    email: &'static [u8],
    time: &'static str,
) -> gix::actor::SignatureRef<'static> {
    gix::actor::SignatureRef {
        name: name.as_bstr(),
        email: email.as_bstr(),
        time,
    }
}

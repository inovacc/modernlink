use git::{HistoryOptions, collect_history};
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

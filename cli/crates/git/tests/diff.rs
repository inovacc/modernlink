use git::{HistoryOptions, collect_history};
use gix::bstr::{BString, ByteSlice};
use tempfile::tempdir;

#[test]
fn history_records_structured_path_changes_and_co_changes() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let signature = signature();
    let initial_tree = tree(&repository, &[("Payment.java", b"class Payment {}")]);
    let first = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "initial\n",
            initial_tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create initial commit")
        .detach();
    let changed_tree = tree(
        &repository,
        &[
            ("Payment.java", b"class Payment { void authorize() {} }"),
            ("Settlement.java", b"class Settlement {}"),
        ],
    );
    let second = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "add settlement\n",
            changed_tree,
            std::iter::once(first),
        )
        .expect("create changed commit")
        .detach();

    let report = collect_history(directory.path(), &HistoryOptions::all())
        .expect("collect history with changes");

    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == first
            && change.path == "Payment.java"
            && change.kind == "addition"
            && change.additions == 1
            && change.deletions == 0
            && change.line_count_status == "measured"
            && change.rename_detection == "disabled"
    }));
    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == second
            && change.path == "Payment.java"
            && change.kind == "modification"
            && change.additions == 1
            && change.deletions == 1
            && change.line_count_status == "measured"
            && change.rename_detection == "disabled"
    }));
    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == second && change.path == "Settlement.java" && change.kind == "addition"
    }));
    assert!(report.co_changes.iter().any(|co_change| {
        co_change.left_path == "Payment.java"
            && co_change.right_path == "Settlement.java"
            && co_change.commit_ids == vec![second.to_string()]
    }));
    assert!(report.knowledge_signals.iter().any(|signal| {
        signal.path == "Payment.java"
            && signal.identity_key == "Historian\u{0}historian@example.test"
            && signal.changed_commit_count == 2
    }));
    assert!(report.metrics.iter().any(|metric| {
        metric.subject == "Historian\u{0}historian@example.test"
            && metric.name == "commit-count"
            && metric.numerator == 2
            && metric.denominator == 2
    }));
}

#[test]
fn co_change_cap_records_incomplete_evidence_instead_of_guessing() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let signature = signature();
    let tree = tree(
        &repository,
        &[
            ("Payment.java", b"class Payment {}"),
            ("Settlement.java", b"class Settlement {}"),
        ],
    );
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "two paths\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit");

    let report = collect_history(
        directory.path(),
        &HistoryOptions {
            max_cochange_paths: 1,
            ..HistoryOptions::all()
        },
    )
    .expect("collect capped history");

    assert!(report.co_changes.is_empty());
    assert!(
        report
            .completeness
            .iter()
            .any(|item| item.code == "cochange-path-cap-reached")
    );
}

#[test]
fn history_ignores_directory_entries_and_records_nested_file_changes() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let signature = signature();
    let nested_tree = nested_tree(&repository, "src", "Payment.java", b"class Payment {}");
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "nested source\n",
            nested_tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create nested fixture commit");

    let report =
        collect_history(directory.path(), &HistoryOptions::all()).expect("collect nested history");

    assert!(
        report
            .path_changes
            .iter()
            .any(|change| change.path == "src/Payment.java")
    );
    assert!(
        !report
            .path_changes
            .iter()
            .any(|change| change.path == "src")
    );
}

#[test]
fn merge_path_delta_uses_only_the_first_parent() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let signature = signature();
    let base_tree = repository.empty_tree().id().detach();
    let base = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "base\n",
            base_tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create base")
        .detach();
    let main_tree = tree(&repository, &[("Payment.java", b"class Payment {}")]);
    let main = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "main change\n",
            main_tree,
            std::iter::once(base),
        )
        .expect("create main")
        .detach();
    let feature_tree = tree(&repository, &[("Settlement.java", b"class Settlement {}")]);
    let feature = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/feature",
            "feature change\n",
            feature_tree,
            std::iter::once(base),
        )
        .expect("create feature")
        .detach();
    let merge_tree = tree(
        &repository,
        &[
            ("Payment.java", b"class Payment {}"),
            ("Settlement.java", b"class Settlement {}"),
        ],
    );
    let merge = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "merge\n",
            merge_tree,
            [main, feature],
        )
        .expect("create merge")
        .detach();

    let report =
        collect_history(directory.path(), &HistoryOptions::all()).expect("collect merge history");

    let merge_fact = report
        .commits
        .iter()
        .find(|commit| commit.object_id == merge)
        .expect("merge fact");
    assert_eq!(
        merge_fact.parent_ids,
        vec![main.to_string(), feature.to_string()]
    );
    assert_eq!(merge_fact.diff_parent_policy, "first-parent");
    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == merge && change.path == "Settlement.java" && change.kind == "addition"
    }));
    assert!(
        !report
            .path_changes
            .iter()
            .any(|change| { change.commit_id == merge && change.path == "Payment.java" })
    );
}

fn tree(repository: &gix::Repository, files: &[(&str, &[u8])]) -> gix::ObjectId {
    let mut entries = files
        .iter()
        .map(|(path, contents)| gix::objs::tree::Entry {
            mode: gix::objs::tree::EntryKind::Blob.into(),
            filename: BString::from(*path),
            oid: repository
                .write_blob(*contents)
                .expect("write blob")
                .detach(),
        })
        .collect::<Vec<_>>();
    entries.sort();
    repository
        .write_object(gix::objs::Tree { entries })
        .expect("write tree")
        .detach()
}

fn nested_tree(
    repository: &gix::Repository,
    directory: &str,
    filename: &str,
    contents: &[u8],
) -> gix::ObjectId {
    let blob = repository
        .write_blob(contents)
        .expect("write blob")
        .detach();
    let child = repository
        .write_object(gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: BString::from(filename),
                oid: blob,
            }],
        })
        .expect("write child tree")
        .detach();
    repository
        .write_object(gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Tree.into(),
                filename: BString::from(directory),
                oid: child,
            }],
        })
        .expect("write root tree")
        .detach()
}

fn signature() -> gix::actor::SignatureRef<'static> {
    gix::actor::SignatureRef {
        name: b"Historian".as_bstr(),
        email: b"historian@example.test".as_bstr(),
        time: "1 +0000",
    }
}

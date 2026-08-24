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
        change.commit_id == first.to_string()
            && change.path == "Payment.java"
            && change.kind == "addition"
            && change.additions == 1
            && change.deletions == 0
            && change.line_count_status == "measured"
            && change.rename_detection == "disabled"
    }));
    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == second.to_string()
            && change.path == "Payment.java"
            && change.kind == "modification"
            && change.additions == 1
            && change.deletions == 1
            && change.line_count_status == "measured"
            && change.rename_detection == "disabled"
    }));
    assert!(report.path_changes.iter().any(|change| {
        change.commit_id == second.to_string()
            && change.path == "Settlement.java"
            && change.kind == "addition"
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

fn signature() -> gix::actor::SignatureRef<'static> {
    gix::actor::SignatureRef {
        name: b"Historian".as_bstr(),
        email: b"historian@example.test".as_bstr(),
        time: "1 +0000",
    }
}

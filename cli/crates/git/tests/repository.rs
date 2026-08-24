use git::{RefScope, open_repository, resolve_refs};
use gix::bstr::ByteSlice;
use tempfile::tempdir;

#[test]
fn all_scope_records_local_branch_target() {
    let directory = tempdir().expect("temporary directory");
    let repository = gix::init(directory.path()).expect("initialize fixture repository");
    let signature = gix::actor::SignatureRef {
        name: b"ModernLink Test".as_bstr(),
        email: b"test@modernlink.invalid".as_bstr(),
        time: "1 +0000",
    };
    let tree = repository.empty_tree().id().detach();
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "fixture\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit");

    let opened = open_repository(directory.path()).expect("open repository");
    let refs = resolve_refs(&opened, RefScope::All).expect("resolve refs");

    assert!(
        refs.iter()
            .any(|reference| reference.name == "refs/heads/main" && !reference.target_id.is_empty())
    );
}

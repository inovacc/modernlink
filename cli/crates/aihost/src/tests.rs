//! additive mutation-closing tests for `kind_for_path`, the registry
//! dedup/order semantics, `Kind` display, and `Asset::render`.
//!
//! subpackage `modernlink/pkg/aihost/assets/all` (the asset barrel), which is
//! outside this root-package implementation. Without registered domain assets it asserts
//! `len(assets) != 0` and would fail vacuously. Its parity claim ("no rendered
//! asset leaks a secret token; every installable skill/command/agent has a
//! non-blank frontmatter description") is left unverified here.

use crate::registry::test_support::{REGISTRY_TEST_LOCK, reset_registry};
use crate::*;
use std::path::PathBuf;

// ---- helpers ---------------------------------------------------------------

/// Unique + clean temp dir (reproduces Rust `t.TempDir`): the nanos suffix makes
/// it unique across runs even under PID reuse, and `remove_dir_all` first
/// guarantees it is empty (w70 lesson — never assert file counts against a
/// possibly-stale shared dir).
fn unique_temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let p = std::env::temp_dir().join(format!("modernlink-aihost-test-{tag}-{pid}-{nanos}-{n}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

struct FakeTree {
    walk: Vec<(String, Vec<u8>)>,
    manifest: Vec<(String, Vec<u8>)>,
}

impl TreeWriter for FakeTree {
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> Result<()>) -> Result<()> {
        for (p, d) in &self.walk {
            f(p, d)?;
        }
        Ok(())
    }
    fn manifest_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        Ok(self.manifest.clone())
    }
}

fn asset(kind: Kind, path: &str) -> Asset {
    Asset {
        kind,
        path: path.to_string(),
        frontmatter: String::new(),
        body: String::new(),
        created: String::new(),
    }
}

// ---- covered: write_tree_test.rust -------------------------------------------

#[test]
fn write_tree_atomic_writes_and_sweeps() {
    let target = unique_temp_dir("sweep");
    let target_s = target.to_str().unwrap().to_string();

    // Pre-existing stale file under a sweep dir — must be removed.
    let stale = target.join("skills").join("old").join("SKILL.md");
    std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
    std::fs::write(&stale, b"stale").unwrap();

    let tree = FakeTree {
        walk: vec![("skills/new/SKILL.md".to_string(), b"new".to_vec())],
        manifest: vec![(".mcp.json".to_string(), b"{}".to_vec())],
    };
    let n = write_tree_atomic(&tree, &target_s, &["skills"]).unwrap();
    assert_eq!(n, 2, "wrote {n} files, want 2 (1 asset + 1 manifest)");

    assert!(
        target.join("skills").join("new").join("SKILL.md").exists(),
        "new asset not written"
    );
    assert!(target.join(".mcp.json").exists(), "manifest not written");
    assert!(
        !stale.exists(),
        "stale file under sweep dir was not removed"
    );
}

// ---- covered: portable_libraries_test.rust -----------------------------------

#[test]
fn portable_library_skills_shape_and_frontmatter() {
    let libs = portable_library_skills();
    assert_eq!(libs.len(), 2, "want 2 libraries (command + agent)");
    for lib in &libs {
        assert_eq!(lib.kind, Kind::Skill, "{} Kind != Skill", lib.path);
        assert!(
            lib.path.ends_with("/SKILL.md"),
            "{} path is not a SKILL.md",
            lib.path
        );
        // Frontmatter MUST end with a newline (Render appends ---/created).
        assert!(
            lib.frontmatter.ends_with('\n'),
            "{} frontmatter does not end with newline",
            lib.path
        );
        assert!(
            lib.frontmatter.contains("name:") && lib.frontmatter.contains("description:"),
            "{} frontmatter missing name/description",
            lib.path
        );
        assert!(
            lib.body.contains("## Index"),
            "{} body missing Index section",
            lib.path
        );
    }
}

// ---- additive: kind_for_path ----------------------------------------------

#[test]
fn kind_for_path_classifies_by_prefix() {
    assert_eq!(kind_for_path("agents/foo.md"), Kind::Agent);
    assert_eq!(kind_for_path("commands/foo.md"), Kind::Command);
    assert_eq!(kind_for_path("skills/foo/SKILL.md"), Kind::Skill);
    // Unrecognized prefix / no slash / wrong case → Unknown.
    assert_eq!(kind_for_path("hooks/foo.json"), Kind::Unknown);
    assert_eq!(kind_for_path("agents"), Kind::Unknown); // needs trailing slash
    assert_eq!(kind_for_path("Agents/foo.md"), Kind::Unknown); // case-sensitive
    assert_eq!(kind_for_path(""), Kind::Unknown);
    // Prefix must be at the START, not anywhere.
    assert_eq!(kind_for_path("x/agents/foo.md"), Kind::Unknown);
}

// ---- additive: Kind display -----------------------------------------------

#[test]
fn kind_display_strings() {
    assert_eq!(Kind::Command.to_string(), "command");
    assert_eq!(Kind::Agent.to_string(), "agent");
    assert_eq!(Kind::Skill.to_string(), "skill");
    assert_eq!(Kind::Unknown.to_string(), "unknown");
}

// ---- additive: registry dedup / order (mutation targets) ------------------

#[test]
fn register_asset_infers_unknown_kind_from_path() {
    let _g = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry();
    register_asset(&[
        asset(Kind::Unknown, "agents/a.md"),
        asset(Kind::Unknown, "commands/c.md"),
        asset(Kind::Unknown, "skills/s/SKILL.md"),
        asset(Kind::Command, "agents/keep.md"), // explicit kind is NOT overridden
    ]);
    assert!(asset_by_path(Kind::Agent, "agents/a.md").is_some());
    assert!(asset_by_path(Kind::Command, "commands/c.md").is_some());
    assert!(asset_by_path(Kind::Skill, "skills/s/SKILL.md").is_some());
    // explicit Command kind preserved even though path says agents/
    assert!(asset_by_path(Kind::Command, "agents/keep.md").is_some());
    assert!(asset_by_path(Kind::Agent, "agents/keep.md").is_none());
    reset_registry();
}

#[test]
fn register_asset_appends_no_dedup_first_match_wins() {
    let _g = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry();
    let mut first = asset(Kind::Command, "commands/dup.md");
    first.body = "first".to_string();
    let mut second = asset(Kind::Command, "commands/dup.md");
    second.body = "second".to_string();
    register_asset(&[first]);
    register_asset(&[second]);
    // No dedup: both live; AssetByPath returns the FIRST insertion-order match.
    let found = asset_by_path(Kind::Command, "commands/dup.md").unwrap();
    assert_eq!(found.body, "first", "asset_by_path must return first match");
    assert_eq!(
        assets_by_kind(Kind::Command).len(),
        2,
        "both duplicates must remain registered (no dedup)"
    );
    reset_registry();
}

#[test]
fn assets_by_kind_sorted_and_filtered() {
    let _g = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry();
    register_asset(&[
        asset(Kind::Command, "commands/c.md"),
        asset(Kind::Command, "commands/a.md"),
        asset(Kind::Command, "commands/b.md"),
        asset(Kind::Agent, "agents/z.md"),
    ]);
    let cmds = assets_by_kind(Kind::Command);
    let paths: Vec<&str> = cmds.iter().map(|a| a.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["commands/a.md", "commands/b.md", "commands/c.md"],
        "assets_by_kind must be sorted by path and filtered to kind"
    );
    let all = all_assets();
    let all_paths: Vec<&str> = all.iter().map(|a| a.path.as_str()).collect();
    assert_eq!(
        all_paths,
        vec![
            "agents/z.md",
            "commands/a.md",
            "commands/b.md",
            "commands/c.md"
        ],
        "all_assets must be sorted by path across all kinds"
    );
    reset_registry();
}

// ---- additive: Asset::render ----------------------------------------------

fn td() -> TemplateData {
    TemplateData {
        name: "modernlink".to_string(),
        version: "dev".to_string(),
        description: "desc".to_string(),
        mcp_command: "modernlink".to_string(),
        created: String::new(),
    }
}

#[test]
fn render_non_skill_appends_created_trailer() {
    let a = Asset {
        kind: Kind::Command,
        path: "commands/x.md".to_string(),
        frontmatter: "name: x\n".to_string(),
        body: "Hello <%.Name%>".to_string(),
        created: String::new(),
    };
    let out = String::from_utf8(a.render(td()).unwrap()).unwrap();
    assert_eq!(
        out,
        "---\nname: x\n---\nHello modernlink\n\n<!-- created:2026-05-24 -->\n"
    );
}

#[test]
fn render_skill_injects_created_frontmatter_no_trailer() {
    let a = Asset {
        kind: Kind::Skill,
        path: "skills/s/SKILL.md".to_string(),
        frontmatter: "name: s\ndescription: d\n".to_string(),
        body: "# Body".to_string(),
        created: String::new(),
    };
    let out = String::from_utf8(a.render(td()).unwrap()).unwrap();
    assert_eq!(
        out,
        "---\nname: s\ndescription: d\ncreated: 2026-05-24\n---\n# Body"
    );
}

#[test]
fn render_template_data_created_overrides_asset() {
    let a = Asset {
        kind: Kind::Command,
        path: "commands/x.md".to_string(),
        frontmatter: String::new(),
        body: "body".to_string(),
        created: "2020-01-01".to_string(),
    };
    let mut d = td();
    d.created = "2099-12-31".to_string();
    let out = String::from_utf8(a.render(d).unwrap()).unwrap();
    assert!(
        out.ends_with("<!-- created:2099-12-31 -->\n"),
        "rustt: {out:?}"
    );
}

#[test]
fn render_asset_created_fallback_when_data_empty() {
    let a = Asset {
        kind: Kind::Command,
        path: "commands/x.md".to_string(),
        frontmatter: String::new(),
        body: "body".to_string(),
        created: "2021-06-06".to_string(),
    };
    // d.created empty → falls back to the asset's own Created.
    let out = String::from_utf8(a.render(td()).unwrap()).unwrap();
    assert!(
        out.ends_with("<!-- created:2021-06-06 -->\n"),
        "rustt: {out:?}"
    );
}

#[test]
fn render_unknown_template_field_errors() {
    let a = Asset {
        kind: Kind::Command,
        path: "commands/x.md".to_string(),
        frontmatter: String::new(),
        body: "<%.Nope%>".to_string(),
        created: String::new(),
    };
    assert!(a.render(td()).is_err(), "unknown template field must error");
}

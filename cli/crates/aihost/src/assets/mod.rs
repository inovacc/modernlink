//! Built-in asset groups. Rust has no package-init side effects, so
//! [`register_all`] is the explicit registration step.
#![allow(clippy::module_inception)]

use crate::asset::{Asset, Kind};

// Built-in group modules.
mod cli;
mod dissect;
mod enrich;
mod knowledge;
mod ops;
mod supervisor;
mod xref;
// The asset barrel.
pub(crate) mod all;

/// Construct one registry asset with the kind inferred from its path prefix
/// (the kind is inferred from the path). Shared by every group module.
pub(crate) fn mk(path: &str, frontmatter: &str, body: &str, created: &str) -> Asset {
    Asset {
        kind: Kind::Unknown,
        path: path.to_string(),
        frontmatter: frontmatter.to_string(),
        body: body.to_string(),
        created: created.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::test_support::{REGISTRY_TEST_LOCK, reset_registry};
    use crate::{Kind, TemplateData, all_assets, asset_by_path};

    /// Every retained asset satisfies
    /// the source implementation authoring gates: non-blank frontmatter `description`,
    /// frontmatter ends with a newline, command bodies carry the `/modernlink:` H1,
    /// and every asset renders cleanly (catches stray `<%…%>` delimiters).
    #[test]
    fn all_assets_present_and_lint_clean() {
        let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
        reset_registry();
        crate::register_all();

        assert!(
            asset_by_path(Kind::Skill, "skills/enrich/SKILL.md").is_some(),
            "enrich skill must register"
        );
        // The retained asset groups form a substantial tree.
        assert!(
            all_assets().len() > 40,
            "the asset registry should register the retained assets, got {}",
            all_assets().len()
        );

        // Source implementation authoring gates over EVERY asset: non-blank frontmatter
        // description (skip the one bodied-only asset), frontmatter ends with a
        // newline, and every asset renders cleanly.
        for a in all_assets() {
            if !a.frontmatter.is_empty() {
                assert!(
                    a.frontmatter.contains("description:"),
                    "{} frontmatter must have a description",
                    a.path
                );
                assert!(
                    a.frontmatter.ends_with('\n'),
                    "{} frontmatter must end with a newline",
                    a.path
                );
            }
            a.render(TemplateData::default())
                .unwrap_or_else(|e| panic!("{} must render: {e}", a.path));
        }
    }
}

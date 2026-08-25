use crate::asset::{Asset, Kind};
use crate::registry::assets_by_kind;
use std::fmt::Write as _;

/// Slash path of the synthesized command-library skill.
pub const COMMAND_LIBRARY_SKILL_PATH: &str = "skills/modernlink-command-library/SKILL.md";
/// Slash path of the synthesized agent-library skill.
pub const AGENT_LIBRARY_SKILL_PATH: &str = "skills/modernlink-agent-library/SKILL.md";

/// Return the synthesized command- and agent-library skills. They are NOT in
/// [`crate::all_assets`] (derived, not registered); hosts opt in by yielding
/// them from `Walk`.
pub fn portable_library_skills() -> Vec<Asset> {
    vec![
        portable_library_skill(
            Kind::Command,
            COMMAND_LIBRARY_SKILL_PATH,
            "modernlink-command-library",
            "Use bundled modernlink slash-command prompts as portable workflow references.",
            "Command Library",
            "Use these entries when the user asks for a slash-command style workflow. Read the matching command asset by name, adapt its instructions to the current host, and execute the workflow with the active tools available in this session.",
        ),
        portable_library_skill(
            Kind::Agent,
            AGENT_LIBRARY_SKILL_PATH,
            "modernlink-agent-library",
            "Use bundled modernlink agent prompts as portable subagent-style references.",
            "Agent Library",
            "Use these entries when the task benefits from a specialist role. Read the matching agent prompt by name, apply its role, inputs, workflow, output contract, and safety rules in the current host, and adapt host-specific tool names where needed.",
        ),
    ]
}

fn portable_library_skill(
    kind: Kind,
    asset_path: &str,
    skill_name: &str,
    description: &str,
    title: &str,
    guidance: &str,
) -> Asset {
    let mut assets = assets_by_kind(kind);
    // Redundant re-sort mirroring the source implementation (AssetsByKind already sorts).
    assets.sort_by(|a, b| a.path.cmp(&b.path));

    let mut b = String::new();
    let _ = write!(b, "# {title}\n\n{guidance}\n\n");
    b.push_str("## Index\n\n");
    for a in &assets {
        let name = trim_ext(base(&a.path));
        let mut desc = frontmatter_description(&a.frontmatter);
        if desc.is_empty() {
            desc = "No description provided.".to_string();
        }
        let _ = writeln!(b, "- `{}` (`{}`) - {}", name, a.path, desc);
    }
    if assets.is_empty() {
        b.push_str("- No assets are registered for this library.\n");
    }
    b.push_str("\n## Adaptation Rules\n\n");
    b.push_str("- Keep the source prompt's behavioral contract, but map Claude-only command, agent, and tool syntax onto this host's available tools.\n");
    b.push_str("- Prefer the active modernlink MCP server (tools named `mcp__modernlink__*` / `modernlink_*`) when available.\n");
    b.push_str("- If a source prompt references a bundled workflow, reference, or template path, treat it as bundled prompt material and load only the parts needed for the current task.\n");

    // Frontmatter ends with '\n' — required so Render closes the block correctly.
    Asset {
        kind: Kind::Skill,
        path: asset_path.to_string(),
        frontmatter: format!("name: {skill_name}\ndescription: {description}\n"),
        body: b,
        created: String::new(),
    }
}

/// `path.Base` over a slash path: the final element (no trailing-slash inputs here).
fn base(p: &str) -> &str {
    match p.rfind('/') {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

/// `strings.TrimSuffix(base, path.Ext(base))`: drop the extension (from the last
/// `.` in the final element), if any.
fn trim_ext(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) => &name[..i],
        None => name,
    }
}

/// Extract the `description:` value from an asset's YAML frontmatter.
pub(crate) fn frontmatter_description(fm: &str) -> String {
    for line in fm.split('\n') {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("description:") {
            return rest.trim().trim_matches(['"', '\'']).to_string();
        }
    }
    String::new()
}

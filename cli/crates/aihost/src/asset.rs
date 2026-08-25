//! Asset model, `Kind` classification, and template rendering.
//!
//! source implementation never constructs an out-of-band kind), so it is an `enum`.

use crate::error::{Error, Result};
use std::fmt::Write as _;

/// Kind classifies a portable plugin asset so cross-host packages can query
/// the registry without caring which host originally authored it.
///
/// Closed set — the source implementation never builds an arbitrary/unknown numeric kind — so
/// this is an `enum`, not an open newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Zero value; also the "unrecognized path" result of [`kind_for_path`].
    Unknown,
    /// `commands/<name>.md` — slash command body + frontmatter.
    Command,
    /// `agents/<name>.md` — subagent definition.
    Agent,
    /// `skills/<name>/SKILL.md` — skill instructions.
    Skill,
}

impl Kind {
    /// The lowercase identifier suitable for log lines (Rust `Kind.String()`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Command => "command",
            Kind::Agent => "agent",
            Kind::Skill => "skill",
            Kind::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Custom template delimiters, avoiding clashes with MD `{{...}}` examples.
pub const TEMPLATE_DELIMS_START: &str = "<%";
/// Closing template delimiter.
pub const TEMPLATE_DELIMS_END: &str = "%>";
/// Default `created:` marker date.
pub const DEFAULT_CREATED: &str = "2026-05-24";

/// Feeds [`Asset::render`] at install time so author-side `<%.Name%>`
/// placeholders resolve to the host's published values.
#[derive(Debug, Clone, Default)]
pub struct TemplateData {
    /// Host / plugin name.
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Plugin description.
    pub description: String,
    /// MCP command name.
    pub mcp_command: String,
    /// `created:` date; falls back per-asset when empty.
    pub created: String,
}

/// One portable plugin file (frontmatter + body) classified by [`Kind`].
#[derive(Debug, Clone)]
pub struct Asset {
    /// Asset classification.
    pub kind: Kind,
    /// Forward-slash path, relative to plugin root.
    pub path: String,
    /// YAML between `---` markers, or "".
    pub frontmatter: String,
    /// Markdown body after frontmatter.
    pub body: String,
    /// `YYYY-MM-DD`, falls back to [`DEFAULT_CREATED`].
    pub created: String,
}

impl Asset {
    fn created(&self) -> String {
        if self.created.is_empty() {
            DEFAULT_CREATED.to_string()
        } else {
            self.created.clone()
        }
    }

    /// Produce on-disk bytes. The caller supplies [`TemplateData`] so
    /// host-specific names/versions flow into the published file.
    pub fn render(&self, mut d: TemplateData) -> Result<Vec<u8>> {
        if d.created.is_empty() {
            d.created = self.created();
        }
        let mut out = String::new();
        if !self.frontmatter.is_empty() {
            out.push_str("---\n");
            out.push_str(&render_template(
                &format!("fm:{}", self.path),
                &self.frontmatter,
                &d,
            )?);
            if self.kind == Kind::Skill {
                let fm = &self.frontmatter;
                if !fm.contains("\ncreated:") && !fm.starts_with("created:") {
                    let _ = writeln!(out, "created: {}", d.created);
                }
            }
            out.push_str("---\n");
        }
        out.push_str(&render_template(
            &format!("body:{}", self.path),
            &self.body,
            &d,
        )?);
        if self.kind != Kind::Skill {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            let _ = write!(out, "\n<!-- created:{} -->\n", d.created);
        }
        Ok(out.into_bytes())
    }
}

/// Classify a plugin-tree-relative slash path into a [`Kind`] by prefix.
pub fn kind_for_path(p: &str) -> Kind {
    if p.starts_with("agents/") {
        Kind::Agent
    } else if p.starts_with("commands/") {
        Kind::Command
    } else if p.starts_with("skills/") {
        Kind::Skill
    } else {
        Kind::Unknown
    }
}

/// Minimal `<% ... %>` template renderer used by [`Asset::render`].
///
/// DEVIATION: the source implementation uses Rust `text/template`; the real registered assets use
/// only simple `<%.Field%>` field substitutions, so this reproduces that subset
/// (whitespace-tolerant `.Field` actions over the five [`TemplateData`] fields).
/// Anything richer (pipelines, conditionals) is unsupcovered and errors — FLAGGED.
pub(crate) fn render_template(label: &str, src: &str, d: &TemplateData) -> Result<String> {
    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find(TEMPLATE_DELIMS_START) {
        out.push_str(&rest[..start]);
        let after = &rest[start + TEMPLATE_DELIMS_START.len()..];
        let end = after
            .find(TEMPLATE_DELIMS_END)
            .ok_or_else(|| Error::Template(format!("parse {label}: unclosed action delimiter")))?;
        let action = after[..end].trim();
        out.push_str(resolve_field(label, action, d)?);
        rest = &after[end + TEMPLATE_DELIMS_END.len()..];
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_field<'a>(label: &str, action: &str, d: &'a TemplateData) -> Result<&'a str> {
    match action {
        ".Name" => Ok(&d.name),
        ".Version" => Ok(&d.version),
        ".Description" => Ok(&d.description),
        ".McpCommand" => Ok(&d.mcp_command),
        ".Created" => Ok(&d.created),
        other => Err(Error::Template(format!(
            "render {label}: unsupcovered template action {other:?}"
        ))),
    }
}

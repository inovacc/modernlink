# aihost asset authoring rules

<!-- rev:001 -->

Plugin assets are Rust string literals rendered only during installation.

## Where assets live

- Add or edit literals in `assets/bundle.rs`; do not add asset Markdown files.
- `Kind` is inferred from the `Path` prefix — do NOT set it explicitly:
    - command → `Path: "commands/<name>.md"`
    - agent → `Path: "agents/<name>.md"`
    - skill → `Path: "skills/<name>/SKILL.md"`

## Frontmatter per Kind

- **Command:** `description`, `argument-hint`, `allowed-tools` — NO `name:` key. The `/modernlink:` prefix appears only
  in the Body H1.
- **Agent:** `name`, `description` (no `tools:` key).
- **Skill:** `name`, `description` (the trigger blurb).
- **Every `Frontmatter` raw string MUST end with a newline** (`…\n` before the closing backtick). See MAINTAINERS §3.

## Body rules

- Reference only real MCP tools and real sibling assets — no invented names.
- Backticks inside a raw-string body use `` ` + "`x`" + ` `` seams.
- Commands delegate to an agent; skills may delegate to another skill — do not duplicate logic across assets.

## After editing

Run `cargo fmt --all -- --check`, `cargo test -p aihost`, and
`cargo test -p modernlink-cli`. If you add a path, extend bundle coverage tests.

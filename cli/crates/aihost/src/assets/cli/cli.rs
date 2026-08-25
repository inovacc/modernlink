//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[mk(
        "skills/modernlink-cli/SKILL.md",
        r#"name: modernlink-cli
description: |
  Reference for the modernlink binary's command surface. The plugin no longer
  registers per-verb tools for agents to call directly — every operation now
  runs as an `modernlink <group> <verb>` invocation via Bash. Invoke this skill
  whenever an agent or slash command needs to know which modernlink subcommand
  to run, what a group's available verbs are, or how to keep large command
  output out of the conversation.
"#,
        r#"
# modernlink-cli — command surface reference

modernlink ships as a single Cobra binary. There is no separate tool registry
to look up any more: every capability is a subcommand, invoked directly via
Bash.

## The mapping rule

Any capability that used to be exposed as a per-verb tool follows one
naming convention: **the tool's group and verb become the CLI's group and
verb.** A tool named for a group and a verb maps onto
`modernlink <group> <verb>`. For example, a java-decompile capability becomes
`modernlink java decompile`, a kb-enrich-pending capability becomes
`modernlink kb enrich pending`, and an app-detect capability becomes
`modernlink app detect`. When in doubt, run `modernlink <group> --help` to confirm the
exact verb spelling and flags before invoking it.

## Discovering the surface live

The authoritative command list is always the binary itself, not this
document (subcommands are added over time):

```bash
modernlink --help                # top-level group list
modernlink <group> --help        # verbs available inside a group
modernlink <group> <verb> --help # flags for one verb
```

## Top-level command groups (as of this writing)

| Group | Purpose |
|-------|---------|
| `advinstaller` | Advanced Installer bootstrapper analysis and extraction |
| `android` | APK analysis, decompilation, reverse engineering |
| `app` | Whole-app / cross-cutting operations (detect, dissect, scan, ...) |
| `asar` | Electron ASAR archive operations |
| `bun` | Bun standalone binary analysis and decompilation |
| `bundle` | Bundle reconstruction (webpack, Vite, esbuild, Rollup) |
| `cache` | Parse HTTP cache |
| `capture` | Capture and replay live Electron/Tauri/Android app behavior |
| `cert` | Binary certificate extraction and analysis (PE + ELF) |
| `chromium` | Extract Chromium profile data |
| `completion` | Generate the shell autocompletion script |
| `css` | CSS extraction and analysis |
| `daemon` | Manage the modernlink host-singleton supervisor daemon |
| `db` | Configure the Postgres knowledge catalog |
| `deb` | Debian package analysis and extraction |
| `debug` | Browse and compare debug sessions |
| `dotnet` | .NET application analysis and dependency inspection |
| `dpapi` | Windows DPAPI decryption |
| `extension` | Browser extension forensics |
| `frida` | Generate Frida instrumentation scripts for Android apps |
| `garble` | Rust binary obfuscation analysis (garble detection) |
| `rustversions` | Catalog of Rust releases: artifacts, checksums, dates, CVE posture |
| `hook` | Claude Code lifecycle hook handlers (invoked by hooks.json) |
| `httpshell` | Secure cross-platform command execution over HTTPS |
| `insights` | Self-improvement insights for modernlink itself (local-only) |
| `ios` | iOS IPA package analysis and extraction |
| `ipc` | IPC channel fuzzing |
| `java` | Java archive and class file analysis |
| `jsdeob` | JavaScript deobfuscator and unpacker |
| `kb` | Knowledge base operations (catalog, enrich, drift, gaps, ops, transfer) |
| `leveldb` | Parse LevelDB databases |
| `license` | License validation testing |
| `mcp` | Run the MCP stdio server |
| `msi` / `msix` / `msm` | MSI/MSIX/Merge-Module package analysis and extraction |
| `nodeaddon` | Node.js native addon (.node) reverse engineering |
| `npm` | NPM package analysis and security scanning |
| `plugin` | Install / uninstall / inspect the modernlink AI-host plugin |
| `probe` | Probe an MCP server to enumerate tools, resources, prompts |
| `pyinst` | Python executable analysis (PyInstaller, zipapp) |
| `registry` | Windows registry forensic dumps |
| `rpm` | RPM package analysis and extraction |
| `sourcemap` | JavaScript source map analysis and extraction |
| `store` | Manage the analysis result cache |
| `tpm` | TPM key extraction |
| `uwp` | UWP (MSIX/AppX) application analysis |
| `vcs` | Source Version Control for Knowledge Sources (Git-managed) |
| `wasm` | WebAssembly binary analysis |
| `webview2` | WebView2 host application analysis |
| `winsvc` | Manage Windows services associated with apps |
| `winui` | WinUI 3 application analysis |

## Verbs for the groups agents call most often

- `android` — `info`, `extract`, `static verify|cert|decompile|dex2jar`,
  `tools status|apktool|jadx|retdec|bundletool|adb`
- `java` — `info`, `decompile`, `extract`, `manifest`,
  `beautify`, `compare`
- `dotnet` — `info`, `deps`, `runtime`, `ipc`, `decompile`
- `asar` — `dump`, `extract`, `list`, `search`
- `kb` — `build`, `catalog`, `curated`, `drift`, `enrich`,
  `findings`, `gaps`, `ops`, `transfer`
- `capture` — `start`, `list`, `diff`, `replay`
- `cert` — `info`, `extract`, `verify`, `compare`, `scan`
- `frida` — `generate`, `from-seams`, `run`, `validate`
- `garble` — `detect`, `info`, `strings`, `symbols`, `scan`
- `app` — `detect`, `dissect`, `scan`, `schema`, `forensic`,
  `heuristic`, `inject`, `reconstruct`
- `npm` — `info`, `download`, `analyze`, `deps`, `batch`,
  `diff`, `mcp`, `probe`, `sandbox`

These lists drift as modernlink grows — treat them as a starting point and
confirm with `modernlink <group> --help` before relying on an exact verb or flag
name.

## Context discipline (mandatory)

Every group supports `--json` for machine-parseable output and
`-o`/`--output <dir>` to write results to disk instead of stdout. When a
command's raw output could be large (APK/JAR/bundle contents, KB dumps,
decompiled trees):

1. Prefer `--output <dir>` so bulk output lands on disk, not in the
   conversation.
2. When you do need stdout, add `--json` and pipe through a summarizer
   (e.g. jq, or a short script) rather than printing the raw payload.
3. Never paste a full raw dump into the conversation — summarize counts,
   key findings, and file paths instead.

## Out of scope

- This skill does not execute anything itself — it only tells you which
  `modernlink <group> <verb>` to run via Bash.
- It is not a substitute for `modernlink <group> --help`, which is always the
  ground truth for flags and subcommand spelling.
"#,
        "2026-05-24",
    )]);
}

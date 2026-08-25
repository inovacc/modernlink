# aihost maintainers

<!-- rev:002 -->

These invariants apply to the Rust `aihost` crate.

1. Assets are Rust raw-string literals in `assets/bundle.rs`. Never add agentic-coding Markdown files to the crate and
   never depend on an external plugin directory at compile time or runtime.
2. Frontmatter must end with `\\n`; `Asset::render` closes the block and applies installation-time template data.
3. Hosts implement the small `Host` trait. Optional installation, status, and doctor capabilities remain separate
   traits.
4. Every registered asset must render successfully and have a non-empty description. Add coverage when adding a new
   asset path.
5. The CLI receives rendered bytes from `aihost`; it must not maintain a second copy of plugin content or discover it
   from the filesystem.
6. Keep the crate provider-neutral, Rust-native, and free of legacy package terminology.

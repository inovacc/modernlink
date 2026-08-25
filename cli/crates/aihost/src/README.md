# aihost — runtime plugin packaging

`aihost` owns the ModernLink plugin contract. It stores command, agent, and skill content as Rust literals, renders
template values at installation time, and writes the selected host tree from memory.

## Contract

- `Host` describes a supported AI host.
- `Installer`, `Status`, and `Doctor` are optional host capabilities.
- The shared atomic tree writer handles safe installation and stale-file cleanup.
- `render_plugin` exposes the complete rendered file set to the CLI.

## Assets

`assets/bundle.rs` contains the plain-text Rust literals. `Asset::render` parses frontmatter, substitutes installation
data such as the creation date, and returns bytes. Static manifests and host configuration are also Rust literals and
are emitted only when installation requests them.

## Hosts

| Host             | Surface                                                |
|------------------|--------------------------------------------------------|
| Claude Code      | commands, agents, skills, manifests, and configuration |
| OpenAI Codex CLI | commands, agents, skills, manifests, and configuration |
| Gemini CLI       | commands, agents, skills, and configuration            |

## Adding a host

1. Implement `Host` in a Rust host module.
2. Render the shared bundle with host-specific `TemplateData`.
3. Return only the manifest/configuration files supported by that host.
4. Register the host in the Rust host registry.
5. Add tests for rendered paths, frontmatter, and installation output.

Run `cargo fmt --all -- --check`, `cargo test -p aihost`, and
`cargo test -p modernlink-cli` before submitting changes.

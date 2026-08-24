use harness::{HarnessCapability, HarnessRegistry};

#[test]
fn registry_exposes_verified_descriptors_without_implicit_install_paths() {
    let registry = HarnessRegistry::builtin();
    let codex = registry.get("codex").expect("Codex descriptor");
    let claude = registry.get("claude").expect("Claude descriptor");

    assert!(codex.capabilities.contains(&HarnessCapability::Skills));
    assert!(claude.capabilities.contains(&HarnessCapability::Skills));
    assert!(
        claude
            .capabilities
            .contains(&HarnessCapability::SlashCommands)
    );
    assert!(codex.installation_paths.is_empty());
    assert!(claude.installation_paths.is_empty());
}

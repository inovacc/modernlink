use std::process::Command;

#[test]
fn harness_list_emits_registry_as_structured_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["harness", "list"])
        .output()
        .expect("run modernlink harness list");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let definitions: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("structured JSON output");
    assert!(
        definitions["harnesses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|definition| definition["id"] == "codex")
    );
}

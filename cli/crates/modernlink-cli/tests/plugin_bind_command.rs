use std::{fs, process::Command};

#[test]
fn plugin_bind_writes_a_versioned_binary_pointer() {
    let installation = tempfile::tempdir().expect("temporary installation");
    let plugin_root = installation.path().join("plugin");
    let binary = installation.path().join("modernlink-test-binary");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::write(&binary, b"deterministic binary fixture").expect("binary fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["plugin", "bind", "--plugin-root"])
        .arg(&plugin_root)
        .arg("--binary")
        .arg(&binary)
        .output()
        .expect("run modernlink plugin bind");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pointer_path = plugin_root.join("config/binary-pointer.json");
    let pointer: serde_json::Value =
        serde_json::from_slice(&fs::read(pointer_path).expect("binary pointer"))
            .expect("pointer JSON");
    assert_eq!(pointer["schema_version"], "modernlink.binary-pointer/v1");
    assert_eq!(
        pointer["binary_path"],
        binary.canonicalize().unwrap().to_string_lossy().as_ref()
    );
    assert_eq!(
        pointer["binary_sha256"],
        "sha256:7ba7f243b824c40674283148d754db0a709b9e652a30c5907de421a151d0ff06"
    );
    assert!(pointer["os"].is_string());
    assert!(pointer["arch"].is_string());
}

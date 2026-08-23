use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

fn start_fixture() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let endpoint = format!(
        "http://{}/metadata",
        listener.local_addr().expect("address")
    );
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().expect("fixture request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).expect("read request");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 40\r\nConnection: close\r\n\r\n{\"service\":\"orders\",\"password\":\"secret\"}",
                )
                .expect("write response");
        }
    });
    (endpoint, server)
}

#[test]
fn runtime_commands_validate_probe_and_observe_a_loopback_profile() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let (endpoint, server) = start_fixture();
    let profile = temp.path().join("profile.json");
    fs::write(
        &profile,
        format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"orders","kind":"generic-http","endpoint":"{endpoint}","http_method":"GET"}}"#
        ),
    )
    .expect("profile");

    let binary = env!("CARGO_BIN_EXE_modernlink");
    let validate = Command::new(binary)
        .args(["runtime", "profile", "validate"])
        .arg(&profile)
        .output()
        .expect("validate command");
    assert!(
        validate.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let probe = Command::new(binary)
        .args(["runtime", "probe", "--profile"])
        .arg(&profile)
        .output()
        .expect("probe command");
    assert!(
        probe.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&probe.stderr)
    );

    let observe = Command::new(binary)
        .args(["runtime", "observe", "--profile"])
        .arg(&profile)
        .args(["--depth", "metadata"])
        .output()
        .expect("observe command");
    assert!(
        observe.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&observe.stderr)
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&observe.stdout).expect("snapshot JSON");
    assert_eq!(
        snapshot["schema_version"],
        "modernlink.runtime-observation/v1alpha1"
    );
    assert_eq!(snapshot["evidence"][0]["payload"]["password"], "[REDACTED]");
    assert!(!String::from_utf8_lossy(&observe.stdout).contains("secret"));

    let output_path = temp.path().join("snapshot.json");
    let persisted = Command::new(binary)
        .args(["runtime", "observe", "--profile"])
        .arg(&profile)
        .args(["--depth", "metadata", "--output"])
        .arg(&output_path)
        .output()
        .expect("persisted observe command");
    assert!(
        persisted.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&persisted.stderr)
    );
    assert!(output_path.is_file());
    assert!(
        !String::from_utf8_lossy(&fs::read(&output_path).expect("snapshot file"))
            .contains("secret")
    );

    server.join().expect("fixture exits");
}

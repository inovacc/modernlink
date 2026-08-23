use runtime_observer::{
    ApprovedCredentialProvider, CommandExecutor, CommandLimits, CommandOutput, CredentialProvider,
    CredentialRef, ExternalHelperApproval, KubernetesConnector, ObservationDepth, ObservationError,
    RuntimeConnector, RuntimeProfile, SshConnector, SystemCommandExecutor, TunnelApproval,
    TunnelGuard, kubernetes_port_forward_plan, ssh_local_forward_plan,
};
use std::{
    ffi::OsString,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

#[derive(Default)]
struct RecordingExecutor {
    calls: Mutex<Vec<(PathBuf, Vec<OsString>)>>,
    output: Vec<u8>,
}

impl CommandExecutor for RecordingExecutor {
    fn execute(
        &self,
        executable: &Path,
        args: &[OsString],
        _: CommandLimits,
    ) -> Result<CommandOutput, ObservationError> {
        self.calls
            .lock()
            .expect("calls")
            .push((executable.to_path_buf(), args.to_vec()));
        Ok(CommandOutput::success(self.output.clone()))
    }
}

fn profile(kind: &str, endpoint: &str, target: &str) -> RuntimeProfile {
    RuntimeProfile::from_json(&format!(
        r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"approved-target","kind":"{kind}","endpoint":"{endpoint}","target":"{target}","scope":{{"environment":"test","namespace":"orders","server_group":null}},"authorization":{{"owner":"platform","reference":"CHG-42","expires_at":"2030-01-01T00:00:00Z"}},"http_method":"GET"}}"#
    ))
    .expect("profile")
}

#[test]
fn kubernetes_uses_the_exact_profile_context_and_emits_allowlisted_evidence() {
    let executor = RecordingExecutor {
        output: br#"{"items":[{"kind":"Pod","metadata":{"name":"orders","namespace":"orders","uid":"uid-1","resourceVersion":"7","annotations":{"token":"must-not-escape"}},"status":{"phase":"Running"}}]}"#.to_vec(),
        ..Default::default()
    };
    let connector = KubernetesConnector::new(&executor, PathBuf::from("kubectl-fixture"));
    let profile = profile(
        "kubernetes",
        "https://unrelated.example.invalid",
        "orders-context",
    );

    let snapshot = connector
        .observe_metadata(&profile, ObservationDepth::Metadata, "now")
        .expect("observation");

    let calls = executor.calls.lock().expect("calls");
    let args = calls[0]
        .1
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(args[0..2], ["--context", "orders-context"]);
    assert!(args.iter().any(|arg| arg == "get"));
    assert!(
        !args
            .iter()
            .any(|arg| matches!(arg.as_str(), "apply" | "exec" | "secrets" | "configmaps"))
    );
    assert_eq!(snapshot.evidence.len(), 1);
    assert_eq!(snapshot.evidence[0].payload["phase"], "Running");
    assert!(snapshot.canonical_json().expect("json").contains("uid-1"));
    assert!(
        !snapshot
            .canonical_json()
            .expect("json")
            .contains("must-not-escape")
    );
}

#[test]
fn ssh_uses_the_exact_profile_alias_and_never_emits_process_arguments() {
    let executor = RecordingExecutor {
        output: b"Linux 6.1 x86_64\n--java--\nopenjdk version \"17.0.1\"\n--listeners--\ntcp LISTEN 0 4096 127.0.0.1:8080 0.0.0.0:*\n".to_vec(),
        ..Default::default()
    };
    let connector = SshConnector::new(&executor, PathBuf::from("ssh-fixture"));
    let profile = profile(
        "ssh",
        "https://unrelated.example.invalid",
        "operator@orders-host",
    );

    let snapshot = connector
        .observe_metadata(&profile, ObservationDepth::Metadata, "now")
        .expect("observation");

    let calls = executor.calls.lock().expect("calls");
    let args = calls[0]
        .1
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(args[4], "operator@orders-host");
    assert!(!args[5].contains("ps "));
    assert!(!args[5].contains(" -p"));
    assert_eq!(
        snapshot
            .evidence
            .iter()
            .find(|item| item.source_operation == "os")
            .expect("OS evidence")
            .payload["os"],
        "Linux 6.1 x86_64"
    );
    assert_eq!(
        snapshot
            .evidence
            .iter()
            .find(|item| item.source_operation == "listener")
            .expect("listener evidence")
            .payload["port"],
        8080
    );
}

#[test]
fn helper_is_not_executed_without_an_exact_approval() {
    let executor = RecordingExecutor::default();
    let provider = ApprovedCredentialProvider::new(&executor, None, None);
    let reference = CredentialRef::ExternalHelper {
        command: "helper-fixture".to_owned(),
    };

    assert!(provider.resolve(&reference).is_err());
    assert!(executor.calls.lock().expect("calls").is_empty());

    let mismatch = ExternalHelperApproval::for_executable(PathBuf::from("other-helper"));
    let provider = ApprovedCredentialProvider::new(&executor, Some(mismatch), None);
    assert!(provider.resolve(&reference).is_err());
    assert!(executor.calls.lock().expect("calls").is_empty());
}

#[test]
fn kubernetes_does_not_resolve_an_unused_profile_credential_reference() {
    let executor = RecordingExecutor {
        output: br#"{"items":[]}"#.to_vec(),
        ..Default::default()
    };
    let connector = KubernetesConnector::new(&executor, PathBuf::from("kubectl-fixture"));
    let mut profile = profile(
        "kubernetes",
        "https://unrelated.example.invalid",
        "orders-context",
    );
    profile.credential_ref = Some(CredentialRef::ExternalHelper {
        command: "unapproved-helper".to_owned(),
    });

    connector.probe(&profile).expect("Kubernetes probe");
    let calls = executor.calls.lock().expect("calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, PathBuf::from("kubectl-fixture"));
}

#[test]
fn tunnel_spawn_accepts_only_a_validated_connector_plan_and_reaps_an_immediate_exit() {
    let profile = profile("ssh", "https://unrelated.example.invalid", "orders-host");
    let plan = ssh_local_forward_plan(
        &profile,
        std::env::current_exe().expect("native fixture"),
        15432,
        5432,
        TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST)),
    )
    .expect("safe plan");
    let tunnel = TunnelGuard::spawn(plan).expect("spawn validated plan");
    std::thread::sleep(Duration::from_millis(30));
    tunnel.close().expect("immediate process reaped");
}

#[test]
fn tunnel_plans_encode_only_loopback_bindings() {
    let kubernetes = profile(
        "kubernetes",
        "https://unrelated.example.invalid",
        "orders-context",
    );
    let plan = kubernetes_port_forward_plan(
        &kubernetes,
        PathBuf::from("kubectl-fixture"),
        "orders-api",
        18080,
        8080,
        TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST)),
    )
    .expect("Kubernetes plan");
    assert_eq!(plan.local_bind(), "127.0.0.1");
    assert_eq!(plan.local_port(), 18080);
    assert!(
        kubernetes_port_forward_plan(
            &kubernetes,
            PathBuf::from("kubectl-fixture"),
            "orders-api",
            0,
            8080,
            TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        )
        .is_err()
    );
}

#[test]
fn system_executor_bounds_timeout_and_both_output_streams_with_native_fixtures() {
    let executable = std::env::current_exe().expect("native test executable");
    let limits = CommandLimits::new(Duration::from_millis(100), 64, 64);
    for fixture in [
        "fixture_sleeps",
        "fixture_writes_stdout",
        "fixture_writes_stderr",
    ] {
        let result = SystemCommandExecutor.execute(
            &executable,
            &[
                OsString::from("--exact"),
                OsString::from(fixture),
                OsString::from("--ignored"),
                OsString::from("--nocapture"),
            ],
            limits,
        );
        assert!(result.is_err(), "{fixture}");
        assert!(
            !result
                .expect_err("bounded failure")
                .to_string()
                .contains("xxxxxxxx")
        );
    }
}

#[test]
#[ignore]
fn fixture_sleeps() {
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
#[ignore]
fn fixture_writes_stdout() {
    print!("{}", "x".repeat(4096));
}

#[test]
#[ignore]
fn fixture_writes_stderr() {
    eprint!("{}", "x".repeat(4096));
}

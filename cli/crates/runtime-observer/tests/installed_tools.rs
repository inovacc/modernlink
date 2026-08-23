use runtime_observer::{
    CommandExecutor, CommandOutput, CredentialResolver, CredentialValue, KubernetesConnector,
    ObservationError, RuntimeConnector, RuntimeProfile, SshConnector, SystemCommandExecutor,
    TunnelApproval, TunnelGuard, kubernetes_port_forward_args, ssh_local_forward_args,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Default)]
struct RecordingExecutor {
    calls: Mutex<Vec<(PathBuf, Vec<OsString>)>>,
}

impl CommandExecutor for RecordingExecutor {
    fn execute(
        &self,
        executable: &Path,
        args: &[OsString],
    ) -> Result<CommandOutput, ObservationError> {
        self.calls
            .lock()
            .expect("calls")
            .push((executable.to_path_buf(), args.to_vec()));
        Ok(CommandOutput::success(br#"{\"items\":[]}"#.to_vec()))
    }
}

fn profile(kind: &str, endpoint: &str) -> RuntimeProfile {
    RuntimeProfile::from_json(&format!(
        r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"approved-target","kind":"{kind}","endpoint":"{endpoint}","scope":{{"environment":"test","namespace":"orders","server_group":null}},"authorization":{{"owner":"platform","reference":"CHG-42","expires_at":"2030-01-01T00:00:00Z"}},"http_method":"GET"}}"#
    ))
    .expect("profile")
}

#[test]
fn kubernetes_discovery_emits_only_fixed_get_argv() {
    let executor = RecordingExecutor::default();
    let connector = KubernetesConnector::new(&executor, PathBuf::from("kubectl-fixture"));

    connector
        .probe(&profile("kubernetes", "https://cluster-context.invalid"))
        .expect("discovery result");

    let calls = executor.calls.lock().expect("calls");
    let (_, args) = calls.first().expect("one command");
    let args = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        args,
        vec![
            "--context",
            "cluster-context.invalid",
            "--namespace",
            "orders",
            "get",
            "deployments,statefulsets,daemonsets,pods,services,ingresses,jobs,cronjobs",
            "--output=json",
        ]
    );
    assert!(
        !args
            .iter()
            .any(|arg| matches!(arg.as_str(), "apply" | "exec" | "secrets" | "configmaps"))
    );
}

#[test]
fn ssh_metadata_uses_the_fixed_script_and_safe_options() {
    let executor = RecordingExecutor::default();
    let connector = SshConnector::new(&executor, PathBuf::from("ssh-fixture"));

    connector
        .probe(&profile("ssh", "https://host-alias.invalid"))
        .expect("metadata result");

    let calls = executor.calls.lock().expect("calls");
    let (_, args) = calls.first().expect("one command");
    let args = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        args[0..4],
        ["-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]
    );
    assert_eq!(args[4], "host-alias.invalid");
    assert_eq!(args[5], runtime_observer::SSH_METADATA_SCRIPT);
    assert!(!args[5].contains("$"));
}

#[test]
fn credential_values_are_resolved_without_serialization_or_diagnostic_leaks() {
    let variable = "MODERNLINK_RUNTIME_TEST_CREDENTIAL";
    // Test-local process environment is intentionally isolated by the test runner.
    unsafe { std::env::set_var(variable, "do-not-serialize-me") };
    let resolved = CredentialResolver::environment(variable).expect("environment credential");
    unsafe { std::env::remove_var(variable) };

    assert_eq!(resolved.expose_for_authorized_use(), b"do-not-serialize-me");
    assert!(!format!("{resolved:?}").contains("do-not-serialize-me"));
    assert!(serde_json::to_string(&BTreeMap::from([("reference", variable)])).is_ok());
    let _: CredentialValue = resolved;
}

#[test]
fn tunnel_requires_loopback_approval_and_drop_terminates_a_real_fake_process() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let marker = temp.path().join("started.txt");
    let fixture = temp.path().join("fake-tunnel.cmd");
    fs::write(
        &fixture,
        format!(
            "@echo off\r\necho started > \"{}\"\r\nping 127.0.0.1 -n 30 > nul\r\n",
            marker.display()
        ),
    )
    .expect("fixture script");

    let denied = TunnelApproval::new(false, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert!(TunnelGuard::spawn(&fixture, &[], denied).is_err());
    let non_loopback = TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    assert!(TunnelGuard::spawn(&fixture, &[], non_loopback).is_err());

    let approval = TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST));
    let tunnel = TunnelGuard::spawn(&fixture, &[], approval).expect("spawn fake tunnel");
    for _ in 0..20 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(marker.exists(), "fixture received a real process launch");
    let pid = tunnel.pid();
    drop(tunnel);
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("Get-Process -Id {pid} -ErrorAction SilentlyContinue"),
        ])
        .output()
        .expect("process check");
    assert!(
        status.stdout.is_empty(),
        "tunnel process {pid} survived its guard"
    );
}

#[test]
fn approved_tunnel_builders_are_loopback_only_and_use_fixed_forwarding_argv() {
    let kubernetes_profile = profile("kubernetes", "https://cluster-context.invalid");
    let approval = TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST));
    let kubernetes =
        kubernetes_port_forward_args(&kubernetes_profile, "orders-api", 18080, 8080, approval)
            .expect("Kubernetes port-forward args")
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
    assert_eq!(
        kubernetes,
        vec![
            "--context",
            "cluster-context.invalid",
            "--namespace",
            "orders",
            "--address",
            "127.0.0.1",
            "port-forward",
            "service/orders-api",
            "18080:8080",
        ]
    );

    let ssh_profile = profile("ssh", "https://host-alias.invalid");
    let ssh = ssh_local_forward_args(&ssh_profile, 15432, 5432, approval)
        .expect("SSH local-forward args")
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        ssh,
        vec![
            "-N",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=10",
            "-L",
            "127.0.0.1:15432:127.0.0.1:5432",
            "host-alias.invalid",
        ]
    );
}

#[test]
fn production_executor_runs_a_fixed_argv_without_a_shell() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let output = temp.path().join("argv.txt");
    let fixture = temp.path().join("argv-fixture.cmd");
    fs::write(
        &fixture,
        format!("@echo off\r\necho %~1^|%~2 > \"{}\"\r\n", output.display()),
    )
    .expect("fixture script");

    SystemCommandExecutor
        .execute(
            &fixture,
            &[OsString::from("literal"), OsString::from("two")],
        )
        .expect("fixed argv execution");
    assert_eq!(
        fs::read_to_string(output).expect("argv output").trim(),
        "literal|two"
    );
}

use crate::{
    Capability, CapabilityReport, CommandExecutor, ObservationDepth, ObservationError,
    ObservationSnapshot, RuntimeConnector, RuntimeObservation, RuntimeProfile, TunnelApproval,
};
use std::{ffi::OsString, path::PathBuf};

pub fn kubernetes_port_forward_args(
    profile: &RuntimeProfile,
    service: &str,
    local_port: u16,
    remote_port: u16,
    approval: TunnelApproval,
) -> Result<Vec<OsString>, ObservationError> {
    if !matches!(profile.kind, crate::ConnectorKind::Kubernetes) {
        return Err(ObservationError::InvalidProfile(
            "Kubernetes port-forward requires a kubernetes profile".to_owned(),
        ));
    }
    require_authorized(profile)?;
    approval.validate()?;
    let namespace = profile
        .scope
        .as_ref()
        .and_then(|scope| scope.namespace.as_deref())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ObservationError::InvalidProfile(
                "Kubernetes port-forward requires an approved namespace scope".to_owned(),
            )
        })?;
    if !valid_kubernetes_name(service) || local_port == 0 || remote_port == 0 {
        return Err(ObservationError::Tunnel(
            "Kubernetes tunnel requires a valid service and non-zero ports".to_owned(),
        ));
    }
    Ok(vec![
        "--context".into(),
        target_name(profile)?.into(),
        "--namespace".into(),
        namespace.into(),
        "--address".into(),
        "127.0.0.1".into(),
        "port-forward".into(),
        format!("service/{service}").into(),
        format!("{local_port}:{remote_port}").into(),
    ])
}

fn valid_kubernetes_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

pub struct KubernetesConnector<'a, E: CommandExecutor + ?Sized> {
    executor: &'a E,
    executable: PathBuf,
}

impl<'a, E: CommandExecutor + ?Sized> KubernetesConnector<'a, E> {
    pub fn new(executor: &'a E, executable: PathBuf) -> Self {
        Self {
            executor,
            executable,
        }
    }

    fn discovery_args(profile: &RuntimeProfile) -> Result<Vec<OsString>, ObservationError> {
        require_authorized(profile)?;
        let namespace = profile
            .scope
            .as_ref()
            .and_then(|scope| scope.namespace.as_deref())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ObservationError::InvalidProfile(
                    "Kubernetes observation requires an approved namespace scope".to_owned(),
                )
            })?;
        let context = target_name(profile)?;
        Ok(vec![
            "--context".into(),
            context.into(),
            "--namespace".into(),
            namespace.into(),
            "get".into(),
            "deployments,statefulsets,daemonsets,pods,services,ingresses,jobs,cronjobs".into(),
            "--output=json".into(),
        ])
    }
}

impl<E: CommandExecutor + ?Sized> RuntimeConnector for KubernetesConnector<'_, E> {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError> {
        if !matches!(profile.kind, crate::ConnectorKind::Kubernetes) {
            return Err(ObservationError::InvalidProfile(
                "Kubernetes connector requires a kubernetes profile".to_owned(),
            ));
        }
        let args = Self::discovery_args(profile)?;
        let output = self.executor.execute(&self.executable, &args)?;
        if output.status != 0 {
            return Err(ObservationError::Command(
                "Kubernetes discovery command failed".to_owned(),
            ));
        }
        Ok(CapabilityReport {
            profile_digest: profile.digest()?,
            connector: "kubernetes".to_owned(),
            capabilities: vec![Capability::supported("kubernetes-namespaced-read")],
        })
    }

    fn observe_metadata(
        &self,
        profile: &RuntimeProfile,
        depth: ObservationDepth,
        captured_at: &str,
    ) -> Result<ObservationSnapshot, ObservationError> {
        let report = self.probe(profile)?;
        ObservationSnapshot::from_observation(
            profile,
            report.connector,
            captured_at,
            depth,
            RuntimeObservation {
                capabilities: report.capabilities,
                evidence: Vec::new(),
                redaction_counters: Default::default(),
            },
        )
    }
}

pub(crate) fn target_name(profile: &RuntimeProfile) -> Result<String, ObservationError> {
    reqwest::Url::parse(&profile.endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .filter(|host| !host.is_empty())
        .ok_or_else(|| {
            ObservationError::InvalidProfile(
                "profile endpoint must name an approved target".to_owned(),
            )
        })
}

pub(crate) fn require_authorized(profile: &RuntimeProfile) -> Result<(), ObservationError> {
    if profile.authorization.is_none() {
        return Err(ObservationError::InvalidProfile(
            "cluster and host observation requires an authorization record".to_owned(),
        ));
    }
    Ok(())
}

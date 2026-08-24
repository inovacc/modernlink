use crate::{
    Capability, CapabilityReport, CommandExecutor, CommandLimits, ObservationDepth,
    ObservationError, ObservationSnapshot, RuntimeConnector, RuntimeEvidence, RuntimeObservation,
    RuntimeProfile, TunnelApproval, TunnelPlan,
};
use std::{ffi::OsString, path::PathBuf};

pub fn kubernetes_port_forward_plan(
    profile: &RuntimeProfile,
    executable: PathBuf,
    service: &str,
    local_port: u16,
    remote_port: u16,
    approval: TunnelApproval,
) -> Result<TunnelPlan, ObservationError> {
    profile.validate()?;
    if !matches!(profile.kind, crate::ConnectorKind::Kubernetes) {
        return Err(ObservationError::InvalidProfile(
            "Kubernetes port-forward requires a kubernetes profile".to_owned(),
        ));
    }
    require_authorized(profile)?;
    approval.validate()?;
    let namespace = namespace(profile, "Kubernetes port-forward")?;
    if !valid_kubernetes_name(service) || local_port == 0 || remote_port == 0 {
        return Err(ObservationError::Tunnel(
            "Kubernetes tunnel requires a valid service and non-zero ports".to_owned(),
        ));
    }
    Ok(TunnelPlan::kubernetes(
        executable,
        target(profile)?.to_owned(),
        namespace.to_owned(),
        service.to_owned(),
        local_port,
        remote_port,
    ))
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
        profile.validate()?;
        require_authorized(profile)?;
        let namespace = namespace(profile, "Kubernetes observation")?;
        Ok(vec![
            "--context".into(),
            target(profile)?.into(),
            "--namespace".into(),
            namespace.into(),
            "get".into(),
            "deployments,statefulsets,daemonsets,pods,services,ingresses,jobs,cronjobs".into(),
            "--output=json".into(),
        ])
    }
    fn collect(&self, profile: &RuntimeProfile) -> Result<RuntimeObservation, ObservationError> {
        let output = self.executor.execute(
            &self.executable,
            &Self::discovery_args(profile)?,
            CommandLimits::observation(),
        )?;
        if output.status != 0 {
            return Err(ObservationError::Command(format!(
                "approved kubectl exited with status {}",
                output.status
            )));
        }
        Ok(RuntimeObservation {
            capabilities: vec![Capability::supported("kubernetes-namespaced-read")],
            evidence: parse_items(profile, &output.stdout)?,
            redaction_counters: Default::default(),
        })
    }
}

impl<E: CommandExecutor + ?Sized> RuntimeConnector for KubernetesConnector<'_, E> {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError> {
        profile.validate()?;
        if !matches!(profile.kind, crate::ConnectorKind::Kubernetes) {
            return Err(ObservationError::InvalidProfile(
                "Kubernetes connector requires a kubernetes profile".to_owned(),
            ));
        }
        let observation = self.collect(profile)?;
        Ok(CapabilityReport {
            profile_digest: profile.digest()?,
            connector: "kubernetes".to_owned(),
            capabilities: observation.capabilities,
        })
    }
    fn observe_metadata(
        &self,
        profile: &RuntimeProfile,
        depth: ObservationDepth,
        captured_at: &str,
    ) -> Result<ObservationSnapshot, ObservationError> {
        profile.validate()?;
        if !matches!(profile.kind, crate::ConnectorKind::Kubernetes) {
            return Err(ObservationError::InvalidProfile(
                "Kubernetes connector requires a kubernetes profile".to_owned(),
            ));
        }
        ObservationSnapshot::from_observation(
            profile,
            "kubernetes",
            captured_at,
            depth,
            self.collect(profile)?,
        )
    }
}

fn parse_items(
    profile: &RuntimeProfile,
    bytes: &[u8],
) -> Result<Vec<RuntimeEvidence>, ObservationError> {
    let root: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| {
        ObservationError::Response("Kubernetes discovery returned invalid JSON".to_owned())
    })?;
    let items = root
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ObservationError::Response("Kubernetes discovery response has no item array".to_owned())
        })?;
    if items.len() > 256 {
        return Err(ObservationError::Response(
            "Kubernetes discovery response exceeds item limit".to_owned(),
        ));
    }
    Ok(items
        .iter()
        .filter_map(|item| item_evidence(profile, item))
        .collect())
}

fn item_evidence(profile: &RuntimeProfile, item: &serde_json::Value) -> Option<RuntimeEvidence> {
    let metadata = item.get("metadata")?.as_object()?;
    let name = bounded(metadata.get("name")?.as_str()?)?;
    let namespace = bounded(metadata.get("namespace")?.as_str()?)?;
    let kind = bounded(
        item.get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Unknown"),
    )?;
    let mut payload = serde_json::Map::new();
    payload.insert("kind".to_owned(), kind.clone().into());
    payload.insert("name".to_owned(), name.clone().into());
    payload.insert("namespace".to_owned(), namespace.clone().into());
    for (source, target) in [("uid", "uid"), ("resourceVersion", "resource_version")] {
        if let Some(value) = metadata
            .get(source)
            .and_then(serde_json::Value::as_str)
            .and_then(bounded)
        {
            payload.insert(target.to_owned(), value.into());
        }
    }
    if let Some(phase) = item
        .get("status")
        .and_then(|status| status.get("phase"))
        .and_then(serde_json::Value::as_str)
        .and_then(bounded)
    {
        payload.insert("phase".to_owned(), phase.into());
    }
    Some(RuntimeEvidence {
        id: String::new(),
        collector: "kubernetes".to_owned(),
        target: target(profile).ok()?.to_owned(),
        resource_key: format!("{namespace}/{kind}/{name}"),
        source_operation: "kubectl-get-namespaced-resources".to_owned(),
        payload_digest: String::new(),
        payload: payload.into(),
    })
}

fn bounded(value: &str) -> Option<String> {
    (value.len() <= 256).then(|| value.to_owned())
}
fn namespace<'a>(
    profile: &'a RuntimeProfile,
    operation: &str,
) -> Result<&'a str, ObservationError> {
    profile
        .scope
        .as_ref()
        .and_then(|scope| scope.namespace.as_deref())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ObservationError::InvalidProfile(format!(
                "{operation} requires an approved namespace scope"
            ))
        })
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
pub(crate) fn target(profile: &RuntimeProfile) -> Result<&str, ObservationError> {
    profile.target.as_deref().ok_or_else(|| {
        ObservationError::InvalidProfile("profile requires an exact validated target".to_owned())
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

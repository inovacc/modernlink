use crate::{
    Capability, CapabilityReport, CommandExecutor, CommandLimits, ObservationDepth,
    ObservationError, ObservationSnapshot, RuntimeConnector, RuntimeEvidence, RuntimeObservation,
    RuntimeProfile, TunnelApproval, TunnelPlan,
    kubernetes::{require_authorized, target},
};
use std::{ffi::OsString, path::PathBuf};

pub const SSH_METADATA_SCRIPT: &str = "uname -srm; printf '\\n--java--\\n'; java -version 2>&1 | head -n 1; printf '\\n--listeners--\\n'; ss -ltnH";

pub fn ssh_local_forward_plan(
    profile: &RuntimeProfile,
    executable: PathBuf,
    local_port: u16,
    remote_port: u16,
    approval: TunnelApproval,
) -> Result<TunnelPlan, ObservationError> {
    profile.validate()?;
    if !matches!(profile.kind, crate::ConnectorKind::Ssh) {
        return Err(ObservationError::InvalidProfile(
            "SSH forwarding requires an ssh profile".to_owned(),
        ));
    }
    require_authorized(profile)?;
    approval.validate()?;
    if local_port == 0 || remote_port == 0 {
        return Err(ObservationError::Tunnel(
            "SSH tunnel requires non-zero ports".to_owned(),
        ));
    }
    Ok(TunnelPlan::ssh(
        executable,
        target(profile)?.to_owned(),
        local_port,
        remote_port,
    ))
}

pub struct SshConnector<'a, E: CommandExecutor + ?Sized> {
    executor: &'a E,
    executable: PathBuf,
}
impl<'a, E: CommandExecutor + ?Sized> SshConnector<'a, E> {
    pub fn new(executor: &'a E, executable: PathBuf) -> Self {
        Self {
            executor,
            executable,
        }
    }
    fn metadata_args(profile: &RuntimeProfile) -> Result<Vec<OsString>, ObservationError> {
        profile.validate()?;
        require_authorized(profile)?;
        Ok(vec![
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=10".into(),
            target(profile)?.into(),
            SSH_METADATA_SCRIPT.into(),
        ])
    }
    fn collect(&self, profile: &RuntimeProfile) -> Result<RuntimeObservation, ObservationError> {
        let output = self.executor.execute(
            &self.executable,
            &Self::metadata_args(profile)?,
            CommandLimits::observation(),
        )?;
        if output.status != 0 {
            return Err(ObservationError::Command(format!(
                "approved ssh exited with status {}",
                output.status
            )));
        }
        Ok(RuntimeObservation {
            capabilities: vec![Capability::supported("ssh-fixed-metadata")],
            evidence: parse_metadata(profile, &output.stdout)?,
            redaction_counters: Default::default(),
        })
    }
}
impl<E: CommandExecutor + ?Sized> RuntimeConnector for SshConnector<'_, E> {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError> {
        profile.validate()?;
        if !matches!(profile.kind, crate::ConnectorKind::Ssh) {
            return Err(ObservationError::InvalidProfile(
                "SSH connector requires an ssh profile".to_owned(),
            ));
        }
        let observation = self.collect(profile)?;
        Ok(CapabilityReport {
            profile_digest: profile.digest()?,
            connector: "ssh".to_owned(),
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
        if !matches!(profile.kind, crate::ConnectorKind::Ssh) {
            return Err(ObservationError::InvalidProfile(
                "SSH connector requires an ssh profile".to_owned(),
            ));
        }
        ObservationSnapshot::from_observation(
            profile,
            "ssh",
            captured_at,
            depth,
            self.collect(profile)?,
        )
    }
}

fn parse_metadata(
    profile: &RuntimeProfile,
    bytes: &[u8],
) -> Result<Vec<RuntimeEvidence>, ObservationError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ObservationError::Response("SSH metadata returned non-UTF-8 output".to_owned())
    })?;
    if text.len() > 64 * 1024 {
        return Err(ObservationError::Response(
            "SSH metadata exceeds text limit".to_owned(),
        ));
    }
    let mut records = Vec::new();
    let mut section = "os";
    for line in text.lines() {
        match line {
            "--java--" => {
                section = "java";
                continue;
            }
            "--listeners--" => {
                section = "listeners";
                continue;
            }
            _ => {}
        }
        if line.is_empty() {
            continue;
        }
        match section {
            "os" if records.is_empty() => records.push(evidence(
                profile,
                "os",
                "os",
                serde_json::json!({"os": bounded(line)?}),
            )),
            "java" if !records.iter().any(|item| item.resource_key == "java") => {
                records.push(evidence(
                    profile,
                    "java",
                    "java",
                    serde_json::json!({"java": bounded(line)?}),
                ))
            }
            "listeners" => {
                if let Some(port) = listening_port(line) {
                    records.push(evidence(
                        profile,
                        "listener",
                        &format!("listener/{port}"),
                        serde_json::json!({"protocol":"tcp", "port":port}),
                    ));
                }
            }
            _ => {}
        }
    }
    if records.is_empty() {
        return Err(ObservationError::Response(
            "SSH metadata returned no allowlisted records".to_owned(),
        ));
    }
    Ok(records)
}

fn evidence(
    profile: &RuntimeProfile,
    source_operation: &str,
    resource_key: &str,
    payload: serde_json::Value,
) -> RuntimeEvidence {
    RuntimeEvidence {
        id: String::new(),
        collector: "ssh".to_owned(),
        target: target(profile).unwrap_or_default().to_owned(),
        resource_key: resource_key.to_owned(),
        source_operation: source_operation.to_owned(),
        payload_digest: String::new(),
        payload,
    }
}
fn bounded(value: &str) -> Result<String, ObservationError> {
    if value.len() > 256 {
        Err(ObservationError::Response(
            "SSH metadata field exceeds string limit".to_owned(),
        ))
    } else {
        Ok(value.to_owned())
    }
}
fn listening_port(line: &str) -> Option<u16> {
    let address = line.split_whitespace().nth(3)?;
    address.rsplit(':').next()?.parse().ok()
}

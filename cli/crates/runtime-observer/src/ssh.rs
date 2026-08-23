use crate::{
    Capability, CapabilityReport, CommandExecutor, ObservationDepth, ObservationError,
    ObservationSnapshot, RuntimeConnector, RuntimeObservation, RuntimeProfile, TunnelApproval,
    kubernetes::{require_authorized, target_name},
};
use std::{ffi::OsString, path::PathBuf};

pub const SSH_METADATA_SCRIPT: &str = "uname -srm; printf '\\n--processes--\\n'; ps -eo pid=,ppid=,user=,etimes=,comm=; printf '\\n--services--\\n'; systemctl list-units --type=service --state=running --no-legend --no-pager; printf '\\n--listeners--\\n'; ss -ltnp";

pub fn ssh_local_forward_args(
    profile: &RuntimeProfile,
    local_port: u16,
    remote_port: u16,
    approval: TunnelApproval,
) -> Result<Vec<OsString>, ObservationError> {
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
    Ok(vec![
        "-N".into(),
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-L".into(),
        format!("127.0.0.1:{local_port}:127.0.0.1:{remote_port}").into(),
        target_name(profile)?.into(),
    ])
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
        require_authorized(profile)?;
        Ok(vec![
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=10".into(),
            target_name(profile)?.into(),
            SSH_METADATA_SCRIPT.into(),
        ])
    }
}

impl<E: CommandExecutor + ?Sized> RuntimeConnector for SshConnector<'_, E> {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError> {
        if !matches!(profile.kind, crate::ConnectorKind::Ssh) {
            return Err(ObservationError::InvalidProfile(
                "SSH connector requires an ssh profile".to_owned(),
            ));
        }
        let args = Self::metadata_args(profile)?;
        let output = self.executor.execute(&self.executable, &args)?;
        if output.status != 0 {
            return Err(ObservationError::Command(
                "SSH metadata command failed".to_owned(),
            ));
        }
        Ok(CapabilityReport {
            profile_digest: profile.digest()?,
            connector: "ssh".to_owned(),
            capabilities: vec![Capability::supported("ssh-fixed-metadata")],
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

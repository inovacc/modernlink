use crate::ObservationError;
use command_group::{CommandGroup, GroupChild};
use std::{
    ffi::OsString,
    net::IpAddr,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

#[derive(Debug, Clone, Copy)]
pub struct TunnelApproval {
    approved: bool,
    bind: IpAddr,
}

impl TunnelApproval {
    pub fn new(approved: bool, bind: IpAddr) -> Self {
        Self { approved, bind }
    }
    pub(crate) fn validate(self) -> Result<(), ObservationError> {
        if !self.approved {
            return Err(ObservationError::Tunnel(
                "tunnel requires explicit approval".to_owned(),
            ));
        }
        if !self.bind.is_loopback() {
            return Err(ObservationError::Tunnel(
                "tunnel bind must be loopback-only".to_owned(),
            ));
        }
        Ok(())
    }
}

pub struct TunnelPlan {
    executable: PathBuf,
    local_port: u16,
    remote_port: u16,
    command: TunnelCommand,
}

enum TunnelCommand {
    Kubernetes {
        context: String,
        namespace: String,
        service: String,
    },
    Ssh {
        destination: String,
    },
}

impl TunnelPlan {
    pub(crate) fn kubernetes(
        executable: PathBuf,
        context: String,
        namespace: String,
        service: String,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            executable,
            local_port,
            remote_port,
            command: TunnelCommand::Kubernetes {
                context,
                namespace,
                service,
            },
        }
    }
    pub(crate) fn ssh(
        executable: PathBuf,
        destination: String,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            executable,
            local_port,
            remote_port,
            command: TunnelCommand::Ssh { destination },
        }
    }
    pub fn local_bind(&self) -> &'static str {
        "127.0.0.1"
    }
    pub fn local_port(&self) -> u16 {
        self.local_port
    }
    fn validate(&self) -> Result<(), ObservationError> {
        if self.executable.as_os_str().is_empty() || self.local_port == 0 || self.remote_port == 0 {
            return Err(ObservationError::Tunnel(
                "validated tunnel plan has an invalid executable or port".to_owned(),
            ));
        }
        Ok(())
    }
    fn argv(&self) -> Vec<OsString> {
        match &self.command {
            TunnelCommand::Kubernetes {
                context,
                namespace,
                service,
            } => vec![
                "--context".into(),
                context.into(),
                "--namespace".into(),
                namespace.into(),
                "--address".into(),
                "127.0.0.1".into(),
                "port-forward".into(),
                format!("service/{service}").into(),
                format!("{}:{}", self.local_port, self.remote_port).into(),
            ],
            TunnelCommand::Ssh { destination } => vec![
                "-N".into(),
                "-o".into(),
                "ExitOnForwardFailure=yes".into(),
                "-o".into(),
                "BatchMode=yes".into(),
                "-o".into(),
                "ConnectTimeout=10".into(),
                "-L".into(),
                format!(
                    "127.0.0.1:{}:127.0.0.1:{}",
                    self.local_port, self.remote_port
                )
                .into(),
                destination.into(),
            ],
        }
    }
}

pub struct TunnelGuard {
    child: Option<GroupChild>,
}

impl TunnelGuard {
    /// Starts only a connector-built plan; callers cannot provide raw executable arguments.
    ///
    /// ```compile_fail
    /// use runtime_observer::{TunnelApproval, TunnelGuard};
    /// use std::{net::{IpAddr, Ipv4Addr}, path::Path};
    /// let approval = TunnelApproval::new(true, IpAddr::V4(Ipv4Addr::LOCALHOST));
    /// let _ = TunnelGuard::spawn(Path::new("ssh"), &[], approval);
    /// ```
    pub fn spawn(plan: TunnelPlan) -> Result<Self, ObservationError> {
        plan.validate()?;
        let mut child = Command::new(&plan.executable)
            .args(plan.argv())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .group_spawn()
            .map_err(|error| {
                ObservationError::Tunnel(format!("cannot start approved tunnel: {error}"))
            })?;
        thread::sleep(Duration::from_millis(500));
        if let Some(status) = child.try_wait().map_err(|error| {
            ObservationError::Tunnel(format!("cannot inspect approved tunnel startup: {error}"))
        })? {
            let executable = plan
                .executable
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("approved-tunnel");
            return Err(ObservationError::Tunnel(format!(
                "approved tunnel {executable} exited immediately with status {}",
                status.code().unwrap_or(-1)
            )));
        }
        Ok(Self { child: Some(child) })
    }
    pub fn pid(&self) -> u32 {
        self.child.as_ref().map_or(0, GroupChild::id)
    }
    pub fn close(mut self) -> Result<(), ObservationError> {
        self.terminate()
    }
    fn terminate(&mut self) -> Result<(), ObservationError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                self.child.take();
                Ok(())
            }
            Ok(None) => {
                child.kill().map_err(|error| {
                    ObservationError::Tunnel(format!("cannot terminate tunnel: {error}"))
                })?;
                child.wait().map_err(|error| {
                    ObservationError::Tunnel(format!("cannot reap tunnel: {error}"))
                })?;
                self.child.take();
                Ok(())
            }
            Err(error) => Err(ObservationError::Tunnel(format!(
                "cannot inspect tunnel: {error}"
            ))),
        }
    }
}

impl Drop for TunnelGuard {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

use crate::ObservationError;
use std::{
    ffi::OsString,
    net::IpAddr,
    path::Path,
    process::{Child, Command},
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

pub struct TunnelGuard {
    child: Option<Child>,
}

impl TunnelGuard {
    pub fn spawn(
        executable: &Path,
        args: &[OsString],
        approval: TunnelApproval,
    ) -> Result<Self, ObservationError> {
        approval.validate()?;
        let child = Command::new(executable)
            .args(args)
            .spawn()
            .map_err(|error| {
                ObservationError::Tunnel(format!("cannot start approved tunnel: {error}"))
            })?;
        Ok(Self { child: Some(child) })
    }

    pub fn pid(&self) -> u32 {
        self.child.as_ref().map_or(0, Child::id)
    }

    pub fn close(mut self) -> Result<(), ObservationError> {
        self.terminate()
    }

    fn terminate(&mut self) -> Result<(), ObservationError> {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    child.kill().map_err(|error| {
                        ObservationError::Tunnel(format!("cannot terminate tunnel: {error}"))
                    })?;
                    child.wait().map_err(|error| {
                        ObservationError::Tunnel(format!("cannot reap tunnel: {error}"))
                    })?;
                }
                Err(error) => {
                    return Err(ObservationError::Tunnel(format!(
                        "cannot inspect tunnel: {error}"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Drop for TunnelGuard {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

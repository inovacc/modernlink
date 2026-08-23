use crate::{CredentialRef, ObservationError};
use std::{ffi::OsString, fmt, path::Path, process::Command};

#[derive(Clone, PartialEq, Eq)]
pub struct CredentialValue(Vec<u8>);

impl CredentialValue {
    pub fn expose_for_authorized_use(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for CredentialValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialValue([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn success(stdout: Vec<u8>) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: Vec::new(),
        }
    }
}

pub trait CommandExecutor {
    fn execute(
        &self,
        executable: &Path,
        args: &[OsString],
    ) -> Result<CommandOutput, ObservationError>;
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn execute(
        &self,
        executable: &Path,
        args: &[OsString],
    ) -> Result<CommandOutput, ObservationError> {
        let output = Command::new(executable)
            .args(args)
            .output()
            .map_err(|error| {
                ObservationError::Command(format!(
                    "cannot run approved tool {}: {error}",
                    executable.display()
                ))
            })?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

pub trait PromptProvider {
    fn read_masked(&self) -> Result<CredentialValue, ObservationError>;
}

pub struct CredentialResolver;

impl CredentialResolver {
    pub fn environment(variable: &str) -> Result<CredentialValue, ObservationError> {
        let value = std::env::var(variable).map_err(|_| {
            ObservationError::Credential(format!(
                "credential environment variable {variable} is unavailable"
            ))
        })?;
        if value.is_empty() {
            return Err(ObservationError::Credential(
                "credential environment variable is empty".to_owned(),
            ));
        }
        Ok(CredentialValue(value.into_bytes()))
    }

    pub fn resolve(
        reference: &CredentialRef,
        executor: &dyn CommandExecutor,
        prompt: Option<&dyn PromptProvider>,
    ) -> Result<CredentialValue, ObservationError> {
        match reference {
            CredentialRef::Environment { variable } => Self::environment(variable),
            CredentialRef::ExternalHelper { command } => {
                let output = executor.execute(Path::new(command), &[])?;
                if output.status != 0 {
                    return Err(ObservationError::Credential(
                        "credential helper did not succeed".to_owned(),
                    ));
                }
                let value = String::from_utf8(output.stdout)
                    .map_err(|_| {
                        ObservationError::Credential(
                            "credential helper returned non-UTF-8 output".to_owned(),
                        )
                    })?
                    .trim()
                    .as_bytes()
                    .to_vec();
                if value.is_empty() {
                    return Err(ObservationError::Credential(
                        "credential helper returned an empty credential".to_owned(),
                    ));
                }
                Ok(CredentialValue(value))
            }
            CredentialRef::MaskedPrompt => prompt
                .ok_or_else(|| {
                    ObservationError::Credential(
                        "masked credential prompt requires an approved prompt provider".to_owned(),
                    )
                })?
                .read_masked(),
            CredentialRef::OsKeyring { .. } => Err(ObservationError::Credential(
                "OS keyring resolution is not available in this build".to_owned(),
            )),
        }
    }
}

use crate::{CredentialRef, ObservationError};
use command_group::CommandGroup;
use std::{
    ffi::OsString,
    fmt,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct CommandLimits {
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl CommandLimits {
    pub const fn new(timeout: Duration, stdout_limit: usize, stderr_limit: usize) -> Self {
        Self {
            timeout,
            stdout_limit,
            stderr_limit,
        }
    }

    pub const fn observation() -> Self {
        Self::new(Duration::from_secs(15), 1024 * 1024, 64 * 1024)
    }

    const fn credential() -> Self {
        Self::new(Duration::from_secs(10), 4096, 4096)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// Secret bytes that deliberately do not implement `Serialize`.
///
/// ```compile_fail
/// use runtime_observer::CredentialValue;
/// # let value: CredentialValue = todo!();
/// let _ = serde_json::to_string(&value);
/// ```
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
        limits: CommandLimits,
    ) -> Result<CommandOutput, ObservationError>;
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn execute(
        &self,
        executable: &Path,
        args: &[OsString],
        limits: CommandLimits,
    ) -> Result<CommandOutput, ObservationError> {
        let mut command = Command::new(executable);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.group_spawn().map_err(|error| {
            ObservationError::Command(format!(
                "cannot run approved tool {}: {error}",
                executable.display()
            ))
        })?;
        let stdout = child.inner().stdout.take().ok_or_else(|| {
            ObservationError::Command("approved tool stdout was unavailable".to_owned())
        })?;
        let stderr = child.inner().stderr.take().ok_or_else(|| {
            ObservationError::Command("approved tool stderr was unavailable".to_owned())
        })?;
        let (stdout_sender, stdout_receiver) = mpsc::sync_channel(1);
        let (stderr_sender, stderr_receiver) = mpsc::sync_channel(1);
        let stdout_reader = thread::spawn(move || {
            let _ = stdout_sender.send(read_bounded(stdout, limits.stdout_limit));
        });
        let stderr_reader = thread::spawn(move || {
            let _ = stderr_sender.send(read_bounded(stderr, limits.stderr_limit));
        });
        let deadline = Instant::now() + limits.timeout;
        let mut status = None;
        let mut stdout = None;
        let mut stderr = None;
        while Instant::now() < deadline {
            if status.is_none() {
                status = child.try_wait().map_err(|error| {
                    ObservationError::Command(format!(
                        "cannot inspect approved tool status: {error}"
                    ))
                })?;
            }
            if stdout.is_none() {
                stdout = stdout_receiver.try_recv().ok();
            }
            if stderr.is_none() {
                stderr = stderr_receiver.try_recv().ok();
            }
            if status.is_some() && stdout.is_some() && stderr.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        if status.is_none() || stdout.is_none() || stderr.is_none() {
            child.kill().map_err(|error| {
                ObservationError::Command(format!("cannot terminate approved tool group: {error}"))
            })?;
            let _ = child.wait();
            let cleanup_deadline = Instant::now() + Duration::from_millis(250);
            while (stdout.is_none() || stderr.is_none()) && Instant::now() < cleanup_deadline {
                if stdout.is_none() {
                    stdout = stdout_receiver.try_recv().ok();
                }
                if stderr.is_none() {
                    stderr = stderr_receiver.try_recv().ok();
                }
                thread::sleep(Duration::from_millis(5));
            }
            drop(stdout_reader);
            drop(stderr_reader);
            return Err(ObservationError::Command(
                "approved tool exceeded its bounded deadline".to_owned(),
            ));
        }
        let stdout = stdout
            .unwrap()
            .map_err(|error| ObservationError::Command(error.to_string()))?;
        let stderr = stderr
            .unwrap()
            .map_err(|error| ObservationError::Command(error.to_string()))?;
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        if stdout.exceeded || stderr.exceeded {
            return Err(ObservationError::Command(
                "approved tool exceeded a bounded output limit".to_owned(),
            ));
        }
        Ok(CommandOutput {
            status: status.expect("status checked").code().unwrap_or(-1),
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        })
    }
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn read_bounded(mut reader: impl Read, limit: usize) -> Result<BoundedOutput, ObservationError> {
    let mut bytes = Vec::with_capacity(limit.min(8192));
    let mut exceeded = false;
    let mut buffer = [0_u8; 4096];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            ObservationError::Command(format!("cannot read approved tool output: {error}"))
        })?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        let accepted = remaining.min(read);
        bytes.extend_from_slice(&buffer[..accepted]);
        exceeded |= accepted != read;
    }
    Ok(BoundedOutput { bytes, exceeded })
}

pub trait PromptProvider {
    fn read_masked(&self) -> Result<CredentialValue, ObservationError>;
}
pub trait CredentialProvider {
    fn resolve(&self, reference: &CredentialRef) -> Result<CredentialValue, ObservationError>;
}

#[derive(Debug, Clone)]
pub struct ExternalHelperApproval {
    executable: PathBuf,
}
impl ExternalHelperApproval {
    pub fn for_executable(executable: PathBuf) -> Self {
        Self { executable }
    }
}

pub struct ApprovedCredentialProvider<'a, E: CommandExecutor + ?Sized> {
    executor: &'a E,
    helper_approval: Option<ExternalHelperApproval>,
    prompt: Option<&'a dyn PromptProvider>,
}

impl<'a, E: CommandExecutor + ?Sized> ApprovedCredentialProvider<'a, E> {
    pub fn new(
        executor: &'a E,
        helper_approval: Option<ExternalHelperApproval>,
        prompt: Option<&'a dyn PromptProvider>,
    ) -> Self {
        Self {
            executor,
            helper_approval,
            prompt,
        }
    }
}

impl<E: CommandExecutor + ?Sized> CredentialProvider for ApprovedCredentialProvider<'_, E> {
    fn resolve(&self, reference: &CredentialRef) -> Result<CredentialValue, ObservationError> {
        match reference {
            CredentialRef::Environment { variable } => {
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
            CredentialRef::ExternalHelper { command } => {
                let approved = self
                    .helper_approval
                    .as_ref()
                    .filter(|approval| approval.executable == Path::new(command))
                    .ok_or_else(|| {
                        ObservationError::Credential(
                            "credential helper lacks an exact approval".to_owned(),
                        )
                    })?;
                let output = self.executor.execute(
                    &approved.executable,
                    &[],
                    CommandLimits::credential(),
                )?;
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
            CredentialRef::MaskedPrompt => self
                .prompt
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

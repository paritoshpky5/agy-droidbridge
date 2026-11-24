use serde::Deserialize;
use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore, oneshot};
use tokio::time::timeout;

const AGENT_NAME: &str = "agy-droidbridge-text";
const AGENT: &str = "\
---\n\
name: agy-droidbridge-text\n\
description: Respond to a text request without using tools.\n\
mainAgent: true\n\
subagent: false\n\
tools: []\n\
commandExecutionPolicy: off\n\
---\n\
Answer the request in text without calling tools or delegating.\n";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("proxy is busy")]
    Busy,
    #[error("Antigravity CLI is unavailable")]
    Missing,
    #[error("Antigravity CLI could not start")]
    Spawn,
    #[error("Antigravity CLI timed out")]
    Timeout,
    #[error("Antigravity CLI failed")]
    Failed,
    #[error("Antigravity CLI returned no output")]
    Empty,
    #[error("Antigravity CLI returned malformed output")]
    Malformed,
    #[error("Antigravity CLI output limit exceeded")]
    OutputLimit,
}

pub struct Output {
    pub text: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Clone)]
pub struct Executor(Arc<ExecutorState>);

struct ExecutorState {
    bin: PathBuf,
    deadline: Duration,
    output_limit: usize,
    workspace: PathBuf,
    _temporary_workspace: Option<tempfile::TempDir>,
    pending: Arc<Semaphore>,
    model: Mutex<Option<String>>,
}

impl Executor {
    pub fn new(
        bin: PathBuf,
        deadline: Duration,
        max_pending: usize,
        output_limit: usize,
        workspace: Option<PathBuf>,
    ) -> io::Result<Self> {
        let (workspace, temporary) = match workspace {
            Some(workspace) => (workspace, None),
            None => {
                let temporary = tempfile::Builder::new()
                    .prefix("agy-droidbridge-")
                    .tempdir()?;
                (temporary.path().to_owned(), Some(temporary))
            }
        };
        let workspace = prepare_workspace(&workspace)?;
        Ok(Self(Arc::new(ExecutorState {
            bin,
            deadline,
            output_limit,
            workspace,
            _temporary_workspace: temporary,
            pending: Arc::new(Semaphore::new(max_pending + 1)),
            model: Mutex::new(None),
        })))
    }

    pub async fn run(
        &self,
        prompt: String,
        requested_model: Option<String>,
    ) -> Result<Output, Error> {
        let worker = self.0.clone();
        let pending = worker
            .pending
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let (mut send, reply) = oneshot::channel();
        // The supervised task retains admission and serialization until child cleanup ends.
        tokio::spawn(async move {
            let _pending = pending;
            let mut model = tokio::select! {
                biased;
                () = send.closed() => return,
                lock = timeout(worker.deadline, worker.model.lock()) => match lock {
                    Ok(model) => model,
                    Err(_) => {
                        let _ = send.send(Err(Error::Busy));
                        return;
                    }
                }
            };
            let effective_model = requested_model.as_deref().or(model.as_deref());
            let output = worker.execute(&prompt, effective_model, &mut send).await;
            if output.is_ok() && requested_model.is_some() {
                *model = requested_model;
            }
            let _ = send.send(output);
        });
        reply.await.unwrap_or(Err(Error::Failed))
    }
}

impl ExecutorState {
    async fn execute(
        &self,
        prompt: &str,
        model: Option<&str>,
        reply: &mut oneshot::Sender<Result<Output, Error>>,
    ) -> Result<Output, Error> {
        if reply.is_closed() {
            return Err(Error::Failed);
        }
        let executable = executable(&self.bin).ok_or(Error::Missing)?;
        let mut command = Command::new(executable);
        command
            .args([
                "--agent",
                AGENT_NAME,
                "--disable-slash-commands",
                "--output-format",
                "json",
            ])
            .current_dir(&self.workspace)
            .env_clear()
            .envs(child_environment())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(model) = model {
            command.args(["--model", model]);
        }
        command.args(["-p", prompt]);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Error::Missing
            } else {
                Error::Spawn
            }
        })?;
        #[cfg(unix)]
        let mut process_group = ProcessGroup(child.id().map(|id| id as libc::pid_t));
        let stdout = child.stdout.take().ok_or(Error::Failed)?;
        let stderr = child.stderr.take().ok_or(Error::Failed)?;
        let result = tokio::select! {
            biased;
            () = reply.closed() => Err(Error::Failed),
            result = timeout(self.deadline, async {
                tokio::try_join!(
                    collect(stdout, self.output_limit),
                    collect(stderr, self.output_limit.min(65_536)),
                    async { child.wait().await.map_err(|_| Error::Failed) },
                )
            }) => result.unwrap_or(Err(Error::Timeout)),
        };
        let (stdout, _, status) = match result {
            Ok(result) => result,
            Err(error) => {
                #[cfg(unix)]
                process_group.kill();
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(error);
            }
        };
        #[cfg(unix)]
        {
            process_group.0 = None;
        }
        if !status.success() {
            return Err(Error::Failed);
        }
        parse_output(&stdout)
    }
}

pub fn prepare_workspace(workspace: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(workspace)?;
    let workspace = workspace.canonicalize()?;
    let agent_dir = workspace.join(".agents").join("agents");
    std::fs::create_dir_all(&agent_dir)?;
    let profile = agent_dir.join(format!("{AGENT_NAME}.md"));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&profile)
    {
        Ok(mut file) => file.write_all(AGENT.as_bytes())?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if std::fs::read(&profile)? != AGENT.as_bytes() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "DroidBridge agent file has been edited",
                ));
            }
        }
        Err(error) => return Err(error),
    }
    Ok(workspace)
}

#[cfg(unix)]
struct ProcessGroup(Option<libc::pid_t>);

#[cfg(unix)]
impl ProcessGroup {
    fn kill(&mut self) {
        if let Some(group) = self.0.take() {
            // The PID came from the child made leader of its own process group.
            unsafe { libc::kill(-group, libc::SIGKILL) };
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.kill();
    }
}

async fn collect(reader: impl AsyncRead + Unpin, max: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    reader
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error::Failed)?;
    if bytes.len() > max {
        return Err(Error::OutputLimit);
    }
    Ok(bytes)
}

fn executable(bin: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let windows_bin = {
        let mut bin = bin.to_path_buf();
        if bin.extension().is_none() {
            bin.set_extension("exe");
        }
        if !bin.extension()?.eq_ignore_ascii_case("exe") {
            return None;
        }
        bin
    };
    #[cfg(windows)]
    let bin = windows_bin.as_path();
    if bin.components().count() > 1 || bin.is_absolute() {
        return find_file(bin);
    }
    env::split_paths(&env::var_os("PATH")?)
        .filter(|directory| directory.is_absolute())
        .find_map(|directory| find_file(&directory.join(bin.as_os_str())))
}

fn find_file(path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    let metadata = path.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    Some(path)
}

fn child_environment() -> impl Iterator<Item = (&'static OsStr, std::ffi::OsString)> {
    [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "APPDATA",
        "LOCALAPPDATA",
        "SYSTEMROOT",
        "WINDIR",
        "TEMP",
        "TMP",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "PREFIX",
        "TERMUX_VERSION",
        "TERMUX__PREFIX",
        "TERMUX__ROOTFS_DIR",
        "ANDROID_ROOT",
        "ANDROID_DATA",
        "LD_LIBRARY_PATH",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_STATE_HOME",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ]
    .into_iter()
    .filter_map(|name| env::var_os(name).map(|value| (OsStr::new(name), value)))
}

#[derive(Deserialize)]
struct Envelope {
    status: String,
    response: Option<String>,
    usage: Option<Usage>,
}

#[derive(Default, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

fn parse_output(stdout: &[u8]) -> Result<Output, Error> {
    if stdout.iter().all(u8::is_ascii_whitespace) {
        return Err(Error::Empty);
    }
    let envelope: Envelope = serde_json::from_slice(stdout).map_err(|_| Error::Malformed)?;
    if envelope.status != "SUCCESS" {
        return Err(Error::Failed);
    }
    let text = envelope.response.as_deref().unwrap_or_default().trim();
    if text.is_empty() {
        return Err(Error::Empty);
    }
    let usage = envelope.usage.unwrap_or_default();
    Ok(Output {
        text: text.to_owned(),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_completed_envelope_and_usage() {
        let output = parse_output(br#"{"status":"SUCCESS","response":" hello ","usage":{"input_tokens":2,"output_tokens":1}}"#).unwrap();
        assert_eq!(output.text, "hello");
        assert_eq!(output.input_tokens, Some(2));
        assert_eq!(output.output_tokens, Some(1));
    }

    #[test]
    fn rejects_partial_failed_and_blank_output() {
        assert!(matches!(parse_output(b"not json"), Err(Error::Malformed)));
        assert!(matches!(
            parse_output(br#"{"status":"ERROR","response":"internal details"}"#),
            Err(Error::Failed)
        ));
        assert!(matches!(parse_output(b" \n"), Err(Error::Empty)));
        assert!(matches!(
            parse_output(br#"{"status":"SUCCESS","response":"  "}"#),
            Err(Error::Empty)
        ));
    }
}

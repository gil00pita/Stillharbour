//! Fixed hash-pinned backup processes. No renderer executable/argument interface.
use super::snapshots::SnapshotDestinationInput;
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain},
    library::native,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

const LIMIT: usize = 16 * 1024 * 1024;
const MAX_BINARY: u64 = 150 * 1024 * 1024;
#[derive(Clone)]
pub(crate) struct PinnedHelpers {
    root: PathBuf,
    stopped: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
    #[cfg(target_os = "macos")]
    completions: Arc<AtomicUsize>,
}
#[derive(Clone, Copy)]
pub(crate) enum Helper {
    Restic,
    Rclone,
}
pub(crate) struct HelperOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
}
fn failed(message: &str) -> AppError {
    AppError::new(ErrorCode::WorkerFailed, message)
}
fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn revalidate_chain(path: &Path, chain: &DirectoryChain) -> Result<()> {
    let fresh = DirectoryChain::open(path)?;
    if fresh.identities().len() != chain.identities().len()
        || fresh
            .identities()
            .iter()
            .zip(chain.identities())
            .any(|(a, b)| a.volume != b.volume || a.file != b.file)
    {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "The backup resource/work directory changed native membership.",
        ));
    }
    Ok(())
}
fn host_pin(helper: Helper) -> Result<(&'static str, &'static str)> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    return Ok(match helper {
        Helper::Restic => (
            "restic.exe",
            "b0dd1fd21eea5d8fe1325f55f7118213c21f36de8a261e04c0624a5ab9fd7830",
        ),
        Helper::Rclone => (
            "rclone.exe",
            "41eab3e118d264920b62c3327966c5ece0dc4dd715ee617d9e30450d8ead95a4",
        ),
    });
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok(match helper {
        Helper::Restic => (
            "restic",
            "06582569ff2f10e1935a6f12187c76db02f0bae99e6e54098f2cdc374766d768",
        ),
        Helper::Rclone => (
            "rclone",
            "af0257730173748dc9706ad3c83288bf3a0c5d5e9768fef20235e375e9d835e1",
        ),
    });
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        let _ = helper;
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Pinned backup helpers are unavailable for this OS/architecture.",
        ))
    }
}
fn sha(file: &mut File) -> Result<String> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_BINARY {
        return Err(failed(
            "A backup helper is not an ordinary bounded executable.",
        ));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
impl PinnedHelpers {
    #[cfg(target_os = "macos")]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn cancel_all(&self) {
        self.stopped.store(true, Ordering::Release);
    }
    pub(crate) fn idle(&self) -> bool {
        let idle = self.active.load(Ordering::Acquire) == 0;
        #[cfg(target_os = "macos")]
        return idle && self.completions.load(Ordering::Acquire) == 0;
        #[cfg(not(target_os = "macos"))]
        idle
    }
    pub(crate) fn configure(root: PathBuf) -> Result<Self> {
        if !root.is_absolute() {
            return Err(invalid(
                "Backup resources require a native absolute bundle path.",
            ));
        }
        let this = Self {
            root,
            stopped: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicUsize::new(0)),
            #[cfg(target_os = "macos")]
            completions: Arc::new(AtomicUsize::new(0)),
        };
        this.verified(Helper::Restic)?;
        this.verified(Helper::Rclone)?;
        Ok(this)
    }
    pub(crate) fn verified(&self, helper: Helper) -> Result<(PathBuf, File, DirectoryChain)> {
        let (leaf, expected) = host_pin(helper)?;
        let chain = DirectoryChain::open(&self.root)?;
        let path = self.root.join(leaf);
        let mut file = filesystem::open_readonly(&path)?;
        let identity = filesystem::identity(&file)?;
        let version = native::revision(&file)?;
        if identity.links != 1 || sha(&mut file)? != expected || native::revision(&file)? != version
        {
            return Err(failed(
                "The bundled backup helper failed its pinned integrity check.",
            ));
        }
        let fresh = filesystem::open_readonly(&path)?;
        if filesystem::identity(&fresh)? != identity {
            return Err(failed("A backup helper changed during integrity checking."));
        }
        revalidate_chain(&self.root, &chain)?;
        Ok((path, file, chain))
    }
    pub(crate) fn rclone_program(&self) -> Result<String> {
        let (_helper_path, _file, _chain) = self.verified(Helper::Rclone)?;
        #[cfg(target_os = "macos")]
        let path = std::env::current_exe()?;
        #[cfg(not(target_os = "macos"))]
        let path = _helper_path;
        let text = path.to_str().ok_or_else(|| {
            invalid("The helper path cannot be represented by restic's argument-vector parser.")
        })?;
        if text.contains(['\0', '\r', '\n']) {
            return Err(invalid(
                "The helper bundle path contains unsupported characters.",
            ));
        }
        let quoted = format!("'{}'", text.replace('\'', "'\"'\"'"));
        #[cfg(target_os = "macos")]
        return Ok(format!("{quoted} --stillharbour-rclone-stdio-v1"));
        #[cfg(not(target_os = "macos"))]
        Ok(quoted)
    }
    pub(crate) fn run(
        &self,
        helper: Helper,
        args: &[OsString],
        environment: &BTreeMap<OsString, OsString>,
        cwd: &Path,
        cancel: &AtomicBool,
        timeout: Duration,
    ) -> Result<HelperOutput> {
        self.run_input(helper, args, environment, cwd, cancel, timeout, None)
    }
    // Process authority, deadline and the optional private stdin channel are
    // deliberately explicit at this single native process boundary.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_input(
        &self,
        helper: Helper,
        args: &[OsString],
        environment: &BTreeMap<OsString, OsString>,
        cwd: &Path,
        cancel: &AtomicBool,
        timeout: Duration,
        input: Option<&[u8]>,
    ) -> Result<HelperOutput> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The backup helper service is stopping.",
            ));
        }
        if self
            .active
            .try_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < 4).then_some(active + 1)
            })
            .is_err()
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The bounded backup process queue is full.",
            ));
        }
        let _count = ActiveCount(self.active.clone());
        if input.is_some_and(|bytes| bytes.len() > 4096) {
            return Err(invalid("The scoped helper input exceeds its bound."));
        }
        if args.len() > 64 || args.iter().any(|a| a.as_encoded_bytes().len() > 65536) {
            return Err(invalid("The backend helper request exceeds its bound."));
        }
        let (path, _binary, chain) = self.verified(helper)?;
        // Hold both immutable executable files while restic may start rclone.
        let (_rclone_path, _rclone_binary, _rclone_chain) = self.verified(Helper::Rclone)?;
        let working = DirectoryChain::open(cwd)?;
        let mut command = Command::new(path);
        command
            .args(args)
            .env_clear()
            .envs(environment)
            .current_dir(cwd)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        revalidate_chain(&self.root, &chain)?;
        revalidate_chain(cwd, &working)?;
        #[cfg(target_os = "macos")]
        let mut completion = if matches!(helper, Helper::Restic) {
            let scope = args.windows(2).find_map(|pair| {
                (pair[0].to_str() == Some("--repo"))
                    .then(|| {
                        pair[1]
                            .to_str()
                            .and_then(|repo| repo.strip_prefix("rclone:"))
                    })
                    .flatten()
            });
            if let Some(scope) = scope {
                if scope.len() > 8192 || !scope.starts_with("profile:") {
                    return Err(invalid(
                        "The native rclone scope exceeds its fixed worker policy.",
                    ));
                }
                let lease = super::rclone_completion::begin(cwd, self.completions.clone())?;
                command
                    .envs(lease.env())
                    .env("STILLHARBOUR_RCLONE_HELPER_ROOT", self.root())
                    .env(
                        "STILLHARBOUR_RCLONE_OWNER_PID",
                        std::process::id().to_string(),
                    )
                    .env("STILLHARBOUR_RCLONE_SCOPE", scope);
                Some(lease)
            } else {
                None
            }
        } else {
            None
        };
        if self.stopped.load(Ordering::Acquire) || cancel.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Backup helper canceled before process creation.",
            ));
        }
        let (child, tree) = process_tree::spawn(&mut command)?;
        let mut running = Running {
            child,
            tree,
            done: false,
        };
        let Running { child, tree, .. } = &mut running;
        let over = Arc::new(AtomicBool::new(false));
        let out = read_pipe(
            child
                .stdout
                .take()
                .ok_or_else(|| failed("Helper stdout is unavailable."))?,
            over.clone(),
        );
        let err = read_pipe(
            child
                .stderr
                .take()
                .ok_or_else(|| failed("Helper stderr is unavailable."))?,
            over.clone(),
        );
        if let Some(input) = input {
            use std::io::Write;
            let mut pipe = child
                .stdin
                .take()
                .ok_or_else(|| failed("The private helper input channel is unavailable."))?;
            pipe.write_all(input)?;
            drop(pipe);
        }
        let deadline = Instant::now() + timeout;
        let stopped = loop {
            if cancel.load(Ordering::Acquire) || self.stopped.load(Ordering::Acquire) {
                break Some(AppError::new(
                    ErrorCode::Busy,
                    "Backup helper canceled; inspect the durable job before retry.",
                ));
            }
            if over.load(Ordering::Acquire) {
                break Some(failed("Backup helper output exceeded its bound."));
            }
            if Instant::now() > deadline {
                break Some(AppError::new(
                    ErrorCode::WorkerTimeout,
                    "Backup helper exceeded its time budget; repository state needs inspection.",
                ));
            }
            match process_tree::exited(child) {
                Ok(true) => break None,
                Ok(false) => {}
                Err(error) => break Some(error),
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        // Terminate descendants before reaping the reserved leader PID on Unix.
        tree.stop()?;
        let status = child.wait()?;
        tree.disarm();
        #[cfg(target_os = "macos")]
        if let Some(completion) = &mut completion
            && let Err(error) = completion.close_and_wait(Duration::from_secs(15))
        {
            self.stopped.store(true, Ordering::Release);
            return Err(error);
        }
        let stdout = out
            .join()
            .map_err(|_| failed("Helper stdout reader stopped."))??;
        let _stderr = err
            .join()
            .map_err(|_| failed("Helper stderr reader stopped."))??;
        if let Some(error) = stopped {
            return Err(error);
        }
        if over.load(Ordering::Acquire) {
            return Err(failed("Backup helper output exceeded its bound."));
        }
        running.done = true;
        Ok(HelperOutput {
            code: status.code().unwrap_or(-1),
            stdout,
        })
    }
}
struct ActiveCount(Arc<AtomicUsize>);
impl Drop for ActiveCount {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Running {
    child: Child,
    tree: process_tree::Tree,
    done: bool,
}
impl Drop for Running {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.tree.stop();
            let _ = self.child.wait();
            self.tree.disarm();
        }
    }
}
fn read_pipe(
    mut input: impl Read + Send + 'static,
    over: Arc<AtomicBool>,
) -> std::thread::JoinHandle<Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        input
            .by_ref()
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > LIMIT {
            over.store(true, Ordering::Release);
            bytes.truncate(LIMIT);
        }
        Ok(bytes)
    })
}

/// Nonsecret validated endpoint identity, shown by native credential enrollment.
pub fn credential_context(destination: &SnapshotDestinationInput) -> Result<(String, String)> {
    validate_destination(destination)?;
    let (protocol, host, port, user) = match destination {
        SnapshotDestinationInput::Sftp {
            host, port, user, ..
        } => ("SFTP", host, *port, user),
        SnapshotDestinationInput::Ftps {
            host, port, user, ..
        } => ("FTPS", host, *port, user),
        SnapshotDestinationInput::Ftp {
            host, port, user, ..
        } => ("FTP (unencrypted credentials)", host, *port, user),
        SnapshotDestinationInput::Smb {
            host, port, user, ..
        } => ("SMB", host, *port, user),
        SnapshotDestinationInput::Local { .. } => {
            return Err(invalid(
                "Local repositories do not use transport credentials.",
            ));
        }
    };
    let digest = blake3::hash(
        &serde_json::to_vec(destination).map_err(|_| invalid("Invalid credential destination."))?,
    )
    .to_hex()
    .to_string();
    let trust = match destination {
        SnapshotDestinationInput::Sftp { host_key, .. } => format!(
            "; reviewed public host key BLAKE3 {}",
            blake3::hash(host_key.as_bytes()).to_hex()
        ),
        SnapshotDestinationInput::Ftps {
            ca_certificate_pem: Some(pem),
            ..
        } => format!(
            "; reviewed certificate PEM BLAKE3 {}",
            blake3::hash(pem.as_bytes()).to_hex()
        ),
        SnapshotDestinationInput::Ftps {
            ca_certificate_pem: None,
            ..
        } => "; system certificate trust, hostname verification required".into(),
        _ => String::new(),
    };
    Ok((digest, format!("{protocol}: {user}@{host}:{port}{trust}")))
}
fn line(value: &str, max: usize, empty: bool) -> bool {
    value.len() <= max && (empty || !value.is_empty()) && !value.chars().any(|c| c.is_control())
}
pub(crate) fn validate_destination(destination: &SnapshotDestinationInput) -> Result<()> {
    if let SnapshotDestinationInput::Local { selection_id } = destination {
        return uuid::Uuid::parse_str(selection_id)
            .map(|_| ())
            .map_err(|_| invalid("Invalid native destination selection."));
    }
    let (host, port, user, path) = match destination {
        SnapshotDestinationInput::Sftp {
            host,
            port,
            user,
            remote_path,
            ..
        }
        | SnapshotDestinationInput::Ftps {
            host,
            port,
            user,
            remote_path,
            ..
        }
        | SnapshotDestinationInput::Ftp {
            host,
            port,
            user,
            remote_path,
            ..
        }
        | SnapshotDestinationInput::Smb {
            host,
            port,
            user,
            remote_path,
            ..
        } => (host, *port, user, remote_path),
        _ => unreachable!(),
    };
    if !line(host, 253, false)
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        || port == 0
        || !line(user, 256, false)
        || !line(path, 2048, true)
        || path.split('/').any(|p| matches!(p, "." | ".."))
        || path.contains(['\\', '[', ']'])
    {
        return Err(invalid(
            "The remote endpoint contains an unsupported host, port, user or path.",
        ));
    }
    match destination {
        SnapshotDestinationInput::Sftp { host_key, .. } => {
            let mut pieces = host_key.split_ascii_whitespace();
            let kind = pieces.next();
            let key = pieces.next();
            if kind != Some("ssh-ed25519")
                || key.is_none_or(|k| {
                    k.len() != 68
                        || !k
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
                })
                || pieces.next().is_some()
            {
                return Err(invalid(
                    "SFTP requires an explicitly reviewed ssh-ed25519 public host key.",
                ));
            }
        }
        SnapshotDestinationInput::Ftps {
            ca_certificate_pem: Some(pem),
            ..
        } => {
            if pem.len() > 65536
                || !pem.starts_with("-----BEGIN CERTIFICATE-----")
                || !pem.trim_end().ends_with("-----END CERTIFICATE-----")
            {
                return Err(invalid(
                    "FTPS requires a bounded PEM certificate or the system trust store.",
                ));
            }
        }
        SnapshotDestinationInput::Ftp {
            insecure_acknowledged: false,
            ..
        } => {
            return Err(invalid(
                "Plain FTP requires explicit acknowledgment that transport credentials are unencrypted.",
            ));
        }
        SnapshotDestinationInput::Smb { domain, share, .. }
            if !line(domain, 256, true)
                || !line(share, 256, false)
                || share.contains(['/', '\\']) =>
        {
            return Err(invalid("Invalid SMB domain/share."));
        }
        _ => {}
    }
    Ok(())
}

#[cfg(windows)]
mod process_tree {
    use super::*;
    use std::os::windows::{io::AsRawHandle, process::CommandExt};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_NO_WINDOW, CREATE_SUSPENDED, OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
            },
        },
    };
    pub(super) struct Tree {
        job: HANDLE,
        armed: bool,
    }
    impl Tree {
        pub(super) fn stop(&mut self) -> Result<()> {
            if self.armed && unsafe { TerminateJobObject(self.job, 1) } == 0 {
                return Err(failed("The backup process tree could not be stopped."));
            }
            Ok(())
        }
        pub(super) fn disarm(&mut self) {
            self.armed = false;
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            unsafe {
                if self.armed {
                    TerminateJobObject(self.job, 1);
                }
                CloseHandle(self.job);
            }
        }
    }
    pub(super) fn spawn(command: &mut Command) -> Result<(Child, Tree)> {
        // SAFETY: unnamed private job, exact SDK structure, kill-on-close only.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(failed("A private backup process job could not be created."));
        }
        let mut tree = Tree { job, armed: true };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(failed("Backup process containment is unavailable."));
        }
        let mut child = command
            .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED)
            .spawn()?;
        if unsafe { AssignProcessToJobObject(job, child.as_raw_handle().cast()) } == 0 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(failed("The backup process could not join its private job."));
        }
        let resume = (|| -> Result<()> {
            let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(failed(
                    "The suspended backup thread could not be inspected.",
                ));
            }
            let mut entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            let mut found = None;
            let mut available = unsafe { Thread32First(snapshot, &mut entry) };
            while available != 0 {
                if entry.th32OwnerProcessID == child.id() {
                    found = Some(entry.th32ThreadID);
                    break;
                }
                available = unsafe { Thread32Next(snapshot, &mut entry) };
            }
            unsafe {
                CloseHandle(snapshot);
            }
            let thread = unsafe {
                OpenThread(
                    THREAD_SUSPEND_RESUME,
                    0,
                    found.ok_or_else(|| failed("The suspended backup thread is unavailable."))?,
                )
            };
            if thread.is_null() {
                return Err(failed("The contained backup thread could not be opened."));
            }
            let resumed = unsafe { ResumeThread(thread) };
            unsafe {
                CloseHandle(thread);
            }
            if resumed != 1 {
                return Err(failed(
                    "The contained backup process could not resume exactly once.",
                ));
            }
            Ok(())
        })();
        if let Err(error) = resume {
            let _ = tree.stop();
            let _ = child.wait();
            return Err(error);
        }
        Ok((child, tree))
    }
    pub(super) fn exited(child: &mut Child) -> Result<bool> {
        Ok(child.try_wait()?.is_some())
    }
}
#[cfg(unix)]
mod process_tree {
    use super::*;
    #[cfg(not(target_os = "macos"))]
    use std::os::unix::process::CommandExt;
    pub(super) struct Tree {
        pid: libc::pid_t,
        armed: bool,
    }
    impl Tree {
        pub(super) fn stop(&mut self) -> Result<()> {
            #[cfg(target_os = "macos")]
            let (target, signal) = (self.pid, libc::SIGTERM);
            #[cfg(target_os = "macos")]
            if self.armed {
                let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
                // WNOWAIT proves this PID is still our waitable child and keeps
                // the leader reserved. ECHILD never authorizes signaling a PID.
                if unsafe {
                    libc::waitid(
                        libc::P_PID,
                        self.pid as libc::id_t,
                        information.as_mut_ptr(),
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                } != 0
                {
                    return Err(failed(
                        "Backup supervisor ownership could not be revalidated; no PID was signaled.",
                    ));
                }
            }
            #[cfg(not(target_os = "macos"))]
            let (target, signal) = (-self.pid, libc::SIGKILL);
            if self.armed
                && unsafe { libc::kill(target, signal) } != 0
                && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
            {
                return Err(failed("The backup process group could not be stopped."));
            }
            Ok(())
        }
        pub(super) fn disarm(&mut self) {
            self.armed = false;
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = self.stop();
        }
    }
    pub(super) fn spawn(command: &mut Command) -> Result<(Child, Tree)> {
        #[cfg(target_os = "macos")]
        crate::backup::helper_guard::configure(command)?;
        #[cfg(not(target_os = "macos"))]
        command.process_group(0);
        let child = command.spawn()?;
        let tree = Tree {
            pid: child.id() as libc::pid_t,
            armed: true,
        };
        Ok((child, tree))
    }
    pub(super) fn exited(child: &mut Child) -> Result<bool> {
        let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        // SAFETY: this is our unreaped child; WNOWAIT reserves its PID until group cleanup.
        if unsafe {
            libc::waitid(
                libc::P_PID,
                child.id() as libc::id_t,
                information.as_mut_ptr(),
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: successful waitid initialized the zeroed structure.
        Ok(unsafe { information.assume_init().si_pid() } != 0)
    }
}

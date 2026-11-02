//! Private same-executable stdio role. Dispatch before starting any application
//! runtime. No executable, destination or argument authority comes from IPC.
use super::{
    helper_guard,
    helpers::{Helper, PinnedHelpers},
};
use crate::{
    filesystem::{self, FileIdentity},
    library::native,
};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    mem::{MaybeUninit, size_of},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

pub const ROLE: &str = "--stillharbour-rclone-stdio-v1";
pub(crate) const ROOT_ENV: &str = "STILLHARBOUR_RCLONE_HELPER_ROOT";
pub(crate) const OWNER_ENV: &str = "STILLHARBOUR_RCLONE_OWNER_PID";
pub(crate) const SCOPE_ENV: &str = "STILLHARBOUR_RCLONE_SCOPE";
const FLAGS: [&str; 7] = [
    "serve",
    "restic",
    "--stdio",
    "--transfers=1",
    "--checkers=1",
    "--buffer-size=64k",
    "--multi-thread-streams=0",
];
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Private backup worker authorization failed.",
    )
}
fn convert<T>(value: crate::domain::Result<T>) -> io::Result<T> {
    value.map_err(|_| denied())
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fixture-probes"),
    derive(serde::Serialize, serde::Deserialize)
)]
struct Process {
    pid: libc::pid_t,
    parent: libc::pid_t,
    uid: u32,
    started: (u64, u64),
    image: FileIdentity,
}
fn process(pid: libc::pid_t) -> io::Result<Process> {
    if pid <= 1 {
        return Err(denied());
    }
    let mut info = MaybeUninit::<libc::proc_bsdinfo>::uninit();
    // SAFETY: exact initialized-on-success native structure and bounded buffer.
    if unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size_of::<libc::proc_bsdinfo>() as i32,
        )
    } != size_of::<libc::proc_bsdinfo>() as i32
    {
        return Err(denied());
    }
    // SAFETY: native API returned the complete structure.
    let info = unsafe { info.assume_init() };
    if info.pbi_pid != pid as u32 || info.pbi_uid != unsafe { libc::getuid() } {
        return Err(denied());
    }
    let mut path = [0u8; 4096];
    // SAFETY: native fixed maximum path buffer, no imported path authority.
    let count = unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) };
    if count <= 0 || count as usize >= path.len() {
        return Err(denied());
    }
    let end = path.iter().position(|b| *b == 0).ok_or_else(denied)?;
    let file = convert(filesystem::open_readonly(Path::new(OsStr::from_bytes(
        &path[..end],
    ))))?;
    let image = convert(filesystem::identity(&file))?;
    Ok(Process {
        pid,
        parent: info.pbi_ppid as libc::pid_t,
        uid: info.pbi_uid,
        started: (info.pbi_start_tvsec, info.pbi_start_tvusec),
        image,
    })
}
fn same_image(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}
fn valid_args(args: &[OsString], scope: &OsStr) -> bool {
    let bytes = scope.as_bytes();
    args.len() == FLAGS.len() + 1
        && bytes.len() <= 8192
        && bytes.starts_with(b"profile:")
        && bytes.len() > 8
        && !bytes.iter().any(|b| matches!(b, 0 | b'\r' | b'\n'))
        && args[..FLAGS.len()].iter().zip(FLAGS).all(|(a, b)| a == b)
        && args.last().is_some_and(|a| a == scope)
}
struct Authorization {
    parent: Process,
    guardian: Process,
    owner: Process,
    _app: File,
    _restic: File,
    _rclone: File,
    rclone: PathBuf,
    root: PathBuf,
}
impl Authorization {
    fn fresh(&self) -> io::Result<()> {
        // These native observations authorize no signals to imported PIDs. Only
        // the dedicated C guardian signals its own unreaped child/group.
        if unsafe { libc::getppid() } != self.parent.pid
            || process(self.parent.pid)? != self.parent
            || process(self.guardian.pid)? != self.guardian
            || process(self.owner.pid)? != self.owner
        {
            return Err(denied());
        }
        let helpers = convert(PinnedHelpers::configure(self.root.clone()))?;
        let (_, restic, _) = convert(helpers.verified(Helper::Restic))?;
        let (_, rclone, _) = convert(helpers.verified(Helper::Rclone))?;
        if !same_image(&convert(filesystem::identity(&restic))?, &self.parent.image)
            || convert(filesystem::identity(&rclone))?
                != convert(filesystem::identity(&self._rclone))?
        {
            return Err(denied());
        }
        Ok(())
    }
}
fn authorize(root: PathBuf, owner: libc::pid_t) -> io::Result<Authorization> {
    let helpers = convert(PinnedHelpers::configure(root.clone()))?;
    let (_, restic, _) = convert(helpers.verified(Helper::Restic))?;
    let (rclone_path, rclone, _) = convert(helpers.verified(Helper::Rclone))?;
    let app = convert(filesystem::open_readonly(&std::env::current_exe()?))?;
    let image = convert(filesystem::identity(&app))?;
    // SAFETY: read-only current native lineage.
    let parent = process(unsafe { libc::getppid() })?;
    let guardian = process(parent.parent)?;
    let owner_process = process(guardian.parent)?;
    if owner_process.pid != owner
        || !same_image(&parent.image, &convert(filesystem::identity(&restic))?)
        || !same_image(&guardian.image, &image)
        || !same_image(&owner_process.image, &image)
    {
        return Err(denied());
    }
    let authorization = Authorization {
        parent,
        guardian,
        owner: owner_process,
        _app: app,
        _restic: restic,
        _rclone: rclone,
        rclone: rclone_path,
        root,
    };
    authorization.fresh()?;
    Ok(authorization)
}
struct Worker {
    child: Child,
    done: bool,
}
impl Worker {
    fn stop(&mut self) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        let mut info = MaybeUninit::<libc::siginfo_t>::zeroed();
        // SAFETY: WNOWAIT establishes and reserves our exact direct child. An
        // unexpected ECHILD never authorizes a signal to a recycled process ID.
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id() as libc::id_t,
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if observed != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: exact live or zombie direct child remains reserved above.
        if unsafe { libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM) } != 0
            && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
        {
            return Err(io::Error::last_os_error());
        }
        loop {
            match self.child.wait() {
                Ok(_) => {
                    self.done = true;
                    return Ok(());
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn run(args: Vec<OsString>) -> io::Result<i32> {
    let scope = std::env::var_os(SCOPE_ENV).ok_or_else(denied)?;
    if !valid_args(&args, &scope) {
        return Err(denied());
    }
    let root = PathBuf::from(std::env::var_os(ROOT_ENV).ok_or_else(denied)?);
    let owner = std::env::var(OWNER_ENV)
        .map_err(|_| denied())?
        .parse::<libc::pid_t>()
        .map_err(|_| denied())?;
    let authorization = authorize(root, owner)?;
    #[cfg(feature = "fixture-probes")]
    let fault = probe::WrapperFault::arm()?;
    let revision = convert(native::revision(&authorization._rclone))?;
    let mut command = Command::new(&authorization.rclone);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // Restic already strips RESTIC_*; retain that guarantee independently.
    for (name, _) in std::env::vars_os() {
        if name.as_bytes().starts_with(b"RESTIC_") || name.as_bytes().starts_with(b"STILLHARBOUR_")
        {
            command.env_remove(name);
        }
    }
    let completion = super::rclone_completion::register().map_err(|_| denied())?;
    helper_guard::configure_completion(&mut command, &completion)?;
    authorization.fresh()?;
    if convert(native::revision(&authorization._rclone))? != revision {
        return Err(denied());
    }
    let mut worker = Worker {
        child: command.spawn()?,
        done: false,
    };
    loop {
        #[cfg(feature = "fixture-probes")]
        fault.fire();
        if let Some(status) = worker.child.try_wait()? {
            worker.done = true;
            return Ok(status.code().unwrap_or(125));
        }
        if unsafe { libc::getppid() } != authorization.parent.pid
            || process(authorization.parent.pid).ok().as_ref() != Some(&authorization.parent)
        {
            worker.stop()?;
            return Ok(125);
        }
        thread::sleep(Duration::from_millis(50));
    }
}
/// Returns None for normal startup. Private-role failures produce only a fixed
/// status; paths, credentials and protocol bytes are never logged here.
pub fn dispatch() -> Option<i32> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(OsStr::new(ROLE)) {
        return None;
    }
    Some(run(args.take(10).collect()).unwrap_or(126))
}

#[cfg(test)]
mod tests;

#[cfg(feature = "fixture-probes")]
mod probe;
#[cfg(feature = "fixture-probes")]
pub use probe::guard_probe_dispatch;

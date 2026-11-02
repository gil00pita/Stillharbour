//! macOS post-fork helper ownership. Fixed trusted native commands only; this
//! API is never exposed as a renderer executable/argument interface.
use std::{
    io,
    os::{
        fd::AsRawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::Command,
};

unsafe extern "C" {
    fn stillharbour_backup_parent_guard(
        parent: libc::pid_t,
        max_fd: libc::c_int,
        completion_fd: libc::c_int,
    ) -> libc::c_int;
}

/// The returned Child owns a native supervisor. Stop it with SIGTERM and wait;
/// it kills/reaps its actual helper group. App SIGKILL is detected independently
/// by the supervisor. Never replace normal cancellation with SIGKILL of Child.
pub fn configure(command: &mut Command) -> io::Result<()> {
    // Capture everything requiring parent runtime state BEFORE Command forks.
    // SAFETY: read-only process identity and initialized resource-limit buffer.
    let parent = unsafe { libc::getpid() };
    configure_parent(command, parent, -1)
}

pub(crate) fn configure_completion(
    command: &mut Command,
    completion: &UnixStream,
) -> io::Result<()> {
    let fd = completion.as_raw_fd();
    // SAFETY: borrowed live completion socket; prevent leakage into exec'd
    // rclone, while the native guardian deliberately holds it until reap.
    if fd < 3 || unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: read-only owner identity captured before spawning.
    configure_parent(command, unsafe { libc::getpid() }, fd)
}

fn configure_parent(
    command: &mut Command,
    parent: libc::pid_t,
    completion_fd: libc::c_int,
) -> io::Result<()> {
    let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
    // SAFETY: exact writable libc resource-limit structure.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful getrlimit initialized this fixed structure.
    let limit = unsafe { limit.assume_init() }.rlim_cur;
    if !(8..=1_048_576).contains(&limit) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "The helper descriptor limit cannot be safely bounded.",
        ));
    }
    let max_fd = libc::c_int::try_from(limit).map_err(|_| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "The helper descriptor limit exceeds its safe bound.",
        )
    })?;
    command.process_group(0);
    // SAFETY: the child closure only calls the audited async-signal-safe C
    // supervisor. It allocates/logs nothing. C returns solely into the actual
    // helper's existing std exec path, or an inline OS-error return.
    unsafe {
        command.pre_exec(move || {
            let error = stillharbour_backup_parent_guard(parent, max_fd, completion_fd);
            if error == 0 {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(error))
            }
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;

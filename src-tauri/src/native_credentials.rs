//! Native-only secret entry. No password is returned through the renderer IPC.
use std::sync::atomic::{AtomicBool, Ordering};
use stillharbour_core::{
    backup::{
        credentials::{CredentialPurpose, CredentialStore},
        snapshots::SnapshotDestinationInput,
    },
    domain::{AppError, ErrorCode, Result},
};
use zeroize::Zeroizing;

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn active(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(AppError::new(
            ErrorCode::Busy,
            "Stillharbour is stopping native credential work.",
        ))
    } else {
        Ok(())
    }
}
fn save(
    catalog: &str,
    purpose: CredentialPurpose,
    context: Option<&str>,
    secret: &str,
    cancel: &AtomicBool,
) -> Result<String> {
    active(cancel)?;
    let reference = CredentialStore::store_bound(catalog, purpose, context, secret)?;
    if let Err(error) = active(cancel) {
        CredentialStore::delete_owned(catalog, purpose, &reference)?;
        return Err(error);
    }
    Ok(reference)
}

#[cfg(target_os = "macos")]
struct DialogChild {
    process: std::process::Child,
    reaped: bool,
}
#[cfg(target_os = "macos")]
impl DialogChild {
    fn reap(&mut self) -> std::io::Result<std::process::ExitStatus> {
        loop {
            match self.process.wait() {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                result => {
                    if result.is_ok() {
                        self.reaped = true;
                    }
                    return result;
                }
            }
        }
    }
}
#[cfg(target_os = "macos")]
impl std::ops::Deref for DialogChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.process
    }
}
#[cfg(target_os = "macos")]
impl std::ops::DerefMut for DialogChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}
#[cfg(target_os = "macos")]
impl Drop for DialogChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        loop {
            // SAFETY: this guard is the sole waiter. WNOWAIT keeps its child
            // PID reserved before asking the native supervisor to stop.
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.process.id() as libc::id_t,
                    status.as_mut_ptr(),
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result == 0 {
                // SAFETY: the successful ownership query retained this PID;
                // SIGTERM stops/reaps only its guarded actual helper group.
                unsafe {
                    libc::kill(self.process.id() as libc::pid_t, libc::SIGTERM);
                }
                let _ = self.reap();
                break;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                break;
            }
        }
    }
}

pub(super) fn reenroll(
    target: stillharbour_core::library::snapshots::NativeSnapshotCredentialTarget,
    owner: usize,
    cancel: &AtomicBool,
) -> Result<String> {
    if !cfg!(feature = "backup-workflow") {
        return Err(invalid(
            "Credential recovery requires the backup workflow gate.",
        ));
    }
    let secret = prompt(
        owner,
        &format!(
            "Re-enter the credential for {}. Existing keys and history will be retained.",
            target.label
        ),
        "Stillharbour backup",
        cancel,
    )?;
    save(
        &target.catalog_id,
        target.purpose,
        target.context_digest.as_deref(),
        &secret,
        cancel,
    )
}

pub(super) fn enroll(
    catalog: &str,
    purpose: CredentialPurpose,
    destination: Option<SnapshotDestinationInput>,
    owner: usize,
    cancel: &AtomicBool,
) -> Result<String> {
    if !cfg!(feature = "backup-workflow") {
        return Err(AppError::new(
            ErrorCode::MutationUnavailable,
            "Native backup credential enrollment is disabled until this build's backup gates pass.",
        ));
    }
    let (context, label, user) = match purpose {
        CredentialPurpose::Repository => {
            if destination.is_some() {
                return Err(invalid(
                    "Repository credentials have no transport endpoint.",
                ));
            }
            (None, "Repository password. This is needed to recover encrypted snapshots. It stays in the OS credential store.".to_string(), "Stillharbour repository".to_string())
        }
        CredentialPurpose::Transport => {
            let destination = destination.ok_or_else(|| {
                invalid("Choose the transport endpoint before saving its credential.")
            })?;
            let (digest, label) =
                stillharbour_core::backup::helpers::credential_context(&destination)?;
            let user = match &destination {
                SnapshotDestinationInput::Sftp { user, .. }
                | SnapshotDestinationInput::Ftps { user, .. }
                | SnapshotDestinationInput::Ftp { user, .. }
                | SnapshotDestinationInput::Smb { user, .. } => user.clone(),
                SnapshotDestinationInput::Local { .. } => {
                    return Err(invalid(
                        "Local destinations do not use a transport password.",
                    ));
                }
            };
            (
                Some(digest),
                format!(
                    "Transport password for {label}. This credential will be bound to this endpoint."
                ),
                user,
            )
        }
    };
    let secret = prompt(owner, &label, &user, cancel)?;
    save(catalog, purpose, context.as_deref(), &secret, cancel)
}

#[cfg(windows)]
fn prompt(owner: usize, label: &str, user: &str, cancel: &AtomicBool) -> Result<Zeroizing<String>> {
    active(cancel)?;
    use windows_sys::Win32::{Foundation::ERROR_SUCCESS, Security::Credentials::*};
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }
    let caption = wide("Stillharbour — save backup credential");
    let message = wide(label);
    if message.len() > CREDUI_MAX_MESSAGE_LENGTH as usize
        || user.encode_utf16().count() >= CREDUI_MAX_USERNAME_LENGTH as usize
    {
        return Err(invalid(
            "The credential endpoint label exceeds the native dialog limit.",
        ));
    }
    let target = wide("Stillharbour backup credential enrollment");
    let mut username = Zeroizing::new(vec![0u16; CREDUI_MAX_USERNAME_LENGTH as usize + 1]);
    for (output, input) in username.iter_mut().zip(user.encode_utf16()) {
        *output = input;
    }
    let mut password = Zeroizing::new(vec![0u16; 2049]);
    let info = CREDUI_INFOW {
        cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
        hwndParent: owner as _,
        pszMessageText: message.as_ptr(),
        pszCaptionText: caption.as_ptr(),
        hbmBanner: std::ptr::null_mut(),
    };
    let mut save = 0;
    // SAFETY: fixed live NUL-terminated UTF-16 labels and bounded mutable buffers;
    // the native dialog owns no pointers after this synchronous call returns.
    let result = unsafe {
        CredUIPromptForCredentialsW(
            &info,
            target.as_ptr(),
            std::ptr::null(),
            0,
            username.as_mut_ptr(),
            username.len() as u32,
            password.as_mut_ptr(),
            password.len() as u32,
            &mut save,
            CREDUI_FLAGS_GENERIC_CREDENTIALS
                | CREDUI_FLAGS_ALWAYS_SHOW_UI
                | CREDUI_FLAGS_DO_NOT_PERSIST
                | CREDUI_FLAGS_KEEP_USERNAME,
        )
    };
    active(cancel)?;
    if result != ERROR_SUCCESS {
        return Err(invalid(
            "Native credential entry was canceled or unavailable; no credential was saved.",
        ));
    }
    let end = password
        .iter()
        .position(|unit| *unit == 0)
        .ok_or_else(|| invalid("The native password exceeded its bound."))?;
    String::from_utf16(&password[..end])
        .map(Zeroizing::new)
        .map_err(|_| invalid("The native credential could not be encoded."))
}

#[cfg(target_os = "macos")]
fn prompt(
    _owner: usize,
    label: &str,
    _user: &str,
    cancel: &AtomicBool,
) -> Result<Zeroizing<String>> {
    active(cancel)?;
    use std::{
        io::Read,
        process::{Command, Stdio},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    const SCRIPT: &str = "on run argv\nset requestText to item 1 of argv\nset answer to display dialog requestText with title \"Stillharbour — save backup credential\" default answer \"\" with hidden answer buttons {\"Cancel\", \"Save\"} default button \"Save\" cancel button \"Cancel\"\nreturn text returned of answer\nend run";
    let mut command = Command::new("/usr/bin/osascript");
    command
        .args(["-e", SCRIPT, "--", label])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    stillharbour_core::backup::helper_guard::configure(&mut command)
        .map_err(|_| invalid("Native credential ownership could not be established."))?;
    let process = command
        .spawn()
        .map_err(|_| invalid("The native credential dialog could not open."))?;
    let mut child = DialogChild {
        process,
        reaped: false,
    };
    let exceeded = Arc::new(AtomicBool::new(false));
    let limit = exceeded.clone();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("The native dialog output is unavailable."))?;
    let reader = std::thread::Builder::new()
        .name("native-credential-input".into())
        .spawn(move || -> std::io::Result<Zeroizing<Vec<u8>>> {
            let mut kept = Zeroizing::new(Vec::with_capacity(4097));
            let mut buffer = Zeroizing::new([0u8; 4096]);
            loop {
                let count = stdout.read(&mut *buffer)?;
                if count == 0 {
                    break;
                }
                let remaining = 4097usize.saturating_sub(kept.len());
                kept.extend_from_slice(&buffer[..count.min(remaining)]);
                if count > remaining || kept.len() > 4096 {
                    limit.store(true, Ordering::Release);
                }
            }
            Ok(kept)
        })
        .map_err(|_| invalid("The private native credential reader could not start."))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| invalid("The native dialog error stream is unavailable."))?;
    let errors = std::thread::Builder::new()
        .name("native-credential-errors".into())
        .spawn(move || {
            let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        })
        .map_err(|_| invalid("The native credential error reader could not start."))?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut owned = true;
    let timed_out = loop {
        if cancel.load(Ordering::Acquire)
            || exceeded.load(Ordering::Acquire)
            || Instant::now() >= deadline
        {
            break true;
        }
        // SAFETY: siginfo_t is an OS output record and an all-zero initial value
        // represents no pending child status.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: valid live child PID and output buffer. WNOWAIT keeps its PID
        // reserved until descendants have been stopped and Child::wait reaps it.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                child.id() as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            owned = false;
            break true;
        }
        // SAFETY: waitid initialized the platform siginfo output above.
        if unsafe { info.si_pid() } != 0 {
            break false;
        }
        if cancel.load(Ordering::Acquire)
            || exceeded.load(Ordering::Acquire)
            || Instant::now() >= deadline
        {
            break true;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    // SAFETY: WNOWAIT reserves this sole-owned supervisor PID. SIGTERM asks its
    // native guard to kill/reap only its helper group, including descendants.
    // A failed ownership query never authorizes signaling a possibly reused PID.
    let stopped = owned
        && (unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
    let status = child
        .reap()
        .map_err(|_| invalid("The native credential dialog could not be reaped."))?;
    let mut bytes = reader
        .join()
        .map_err(|_| invalid("The native credential input stopped."))?
        .map_err(|_| invalid("The native credential input could not be read."))?;
    let _ = errors.join();
    if timed_out || !stopped || !status.success() || exceeded.load(Ordering::Acquire) {
        return Err(invalid(
            "Native credential entry was canceled, timed out or exceeded its limit; no credential was saved.",
        ));
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    let raw = std::mem::take(&mut *bytes);
    match String::from_utf8(raw) {
        Ok(secret) => Ok(Zeroizing::new(secret)),
        Err(error) => {
            let _discard = Zeroizing::new(error.into_bytes());
            Err(invalid("The native credential could not be encoded."))
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn prompt(
    _owner: usize,
    _label: &str,
    _user: &str,
    _cancel: &AtomicBool,
) -> Result<Zeroizing<String>> {
    Err(AppError::new(
        ErrorCode::Unsupported,
        "Native credential entry is unavailable on this platform.",
    ))
}

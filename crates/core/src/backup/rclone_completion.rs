//! Private per-restic completion admission. ACK precedes helper spawn; EOF is
//! held by both the wrapper and native guardian until actual rclone is reaped.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::DirectoryChain,
};
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
const PATH_ENV: &str = "STILLHARBOUR_RCLONE_COMPLETION_PATH";
const TOKEN_ENV: &str = "STILLHARBOUR_RCLONE_COMPLETION_TOKEN";
const MAX_CLIENTS: usize = 16;
fn native_confstr_directory(name: libc::c_int) -> Result<PathBuf> {
    let mut buffer = [0u8; 4096];
    // SAFETY: fixed SDK selector and bounded native output buffer.
    let size = unsafe { libc::confstr(name, buffer.as_mut_ptr().cast(), buffer.len()) };
    if size < 2
        || size > buffer.len()
        || buffer[..size].iter().position(|b| *b == 0) != Some(size - 1)
    {
        return Err(failure());
    }
    let path = Path::new(OsStr::from_bytes(&buffer[..size - 1])).to_path_buf();
    if !path.is_absolute() {
        return Err(failure());
    }
    Ok(path)
}
fn native_temp_directory() -> Result<PathBuf> {
    // USER_DIR has no environment fallback. USER_TEMP_DIR may fall back to
    // TMPDIR if dirhelper fails, so require Darwin's exact native sibling T
    // scope before accepting it. Unknown layouts remain unavailable.
    let local = native_confstr_directory(libc::_CS_DARWIN_USER_DIR)?;
    let local = local.canonicalize()?;
    let selected = native_confstr_directory(libc::_CS_DARWIN_USER_TEMP_DIR)?;
    let expected = qualified_temp_path(&local, &selected)?;
    let path = expected.canonicalize()?;
    let metadata = std::fs::symlink_metadata(&path)?;
    // SAFETY: read-only native identity.
    if !path.is_absolute()
        || !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Err(failure());
    }
    DirectoryChain::open(&path)?;
    Ok(path)
}
fn qualified_temp_path(local: &Path, selected: &Path) -> Result<PathBuf> {
    let expected = local.parent().ok_or_else(failure)?.join("T");
    let selected = selected
        .strip_prefix("/private")
        .map_or(selected.to_path_buf(), |short| Path::new("/").join(short));
    let expected_normal = expected
        .strip_prefix("/private")
        .map_or(expected.clone(), |short| Path::new("/").join(short));
    if selected != expected_normal {
        return Err(failure());
    }
    Ok(expected)
}
fn socket_path_fits(path: &Path) -> bool {
    let bytes = path.as_os_str().as_bytes();
    bytes.len() < 104 && !bytes.contains(&0)
}
fn failure() -> AppError {
    AppError::new(
        ErrorCode::WorkerFailed,
        "Native rclone completion could not be certified; helper shutdown remains unconfirmed.",
    )
}
fn native_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Private helper completion authorization failed.",
    )
}
fn same_chain(path: &Path, chain: &DirectoryChain) -> Result<()> {
    let fresh = DirectoryChain::open(path)?;
    if fresh.identities().len() != chain.identities().len()
        || fresh
            .identities()
            .iter()
            .zip(chain.identities())
            .any(|(a, b)| a.volume != b.volume || a.file != b.file)
    {
        return Err(failure());
    }
    Ok(())
}
struct OwnedDirectory {
    tree: Option<tempfile::TempDir>,
    path: PathBuf,
    chain: DirectoryChain,
    _held: File,
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if same_chain(&self.path, &self.chain).is_err()
            && let Some(tree) = self.tree.take()
        {
            let _ = tree.keep();
        }
    }
}
#[derive(Default)]
struct State {
    closed: bool,
    active: usize,
    complete: Option<bool>,
}
type Shared = Arc<(Mutex<State>, Condvar)>;
pub(crate) struct Lease {
    path: PathBuf,
    token: Zeroizing<String>,
    shared: Shared,
    thread: Option<JoinHandle<()>>,
}
impl Lease {
    pub(crate) fn env(&self) -> Vec<(OsString, OsString)> {
        vec![
            (PATH_ENV.into(), self.path.as_os_str().to_owned()),
            (TOKEN_ENV.into(), self.token.as_str().into()),
        ]
    }
    pub(crate) fn close_and_wait(&mut self, timeout: Duration) -> Result<()> {
        let (lock, wake) = &*self.shared;
        let until = Instant::now() + timeout;
        let mut state = lock.lock().map_err(|_| failure())?;
        state.closed = true;
        wake.notify_all();
        loop {
            if let Some(complete) = state.complete {
                drop(state);
                if let Some(thread) = self.thread.take() {
                    thread.join().map_err(|_| failure())?;
                }
                return if complete { Ok(()) } else { Err(failure()) };
            }
            let Some(remaining) = until.checked_duration_since(Instant::now()) else {
                return Err(failure());
            };
            let (next, timed) = wake.wait_timeout(state, remaining).map_err(|_| failure())?;
            state = next;
            if timed.timed_out() && state.complete.is_none() {
                return Err(failure());
            }
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.0.lock() {
            state.closed = true;
            self.shared.1.notify_all();
        }
    }
}
struct Client {
    stream: UnixStream,
    token: [u8; 32],
    received: usize,
    deadline: Instant,
    registered: bool,
}
fn peer(stream: &UnixStream) -> io::Result<()> {
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: live socket and correctly sized writable native UID/GID outputs.
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: read-only current native identity.
    if uid != unsafe { libc::geteuid() } {
        return Err(native_error());
    }
    Ok(())
}
fn serve(
    listener: UnixListener,
    directory: OwnedDirectory,
    token: Zeroizing<String>,
    shared: &Shared,
) -> io::Result<()> {
    let mut clients: Vec<Client> = Vec::new();
    let mut admitted = 0usize;
    loop {
        if same_chain(&directory.path, &directory.chain).is_err() {
            return Err(native_error());
        }
        let closed = shared.0.lock().map_err(|_| native_error())?.closed;
        if !closed {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        admitted += 1;
                        if admitted > MAX_CLIENTS || clients.len() >= MAX_CLIENTS {
                            return Err(native_error());
                        }
                        peer(&stream)?;
                        stream.set_nonblocking(true)?;
                        clients.push(Client {
                            stream,
                            token: [0; 32],
                            received: 0,
                            deadline: Instant::now() + Duration::from_secs(3),
                            registered: false,
                        });
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
        }
        let mut index = 0;
        while index < clients.len() {
            let client = &mut clients[index];
            let mut remove = false;
            if !client.registered {
                if closed || Instant::now() >= client.deadline {
                    remove = true;
                } else {
                    match client.stream.read(&mut client.token[client.received..]) {
                        Ok(0) => remove = true,
                        Ok(count) => {
                            client.received += count;
                            if client.received == 32 {
                                if client.token.as_slice() != token.as_bytes() {
                                    remove = true;
                                } else {
                                    let mut state = shared.0.lock().map_err(|_| native_error())?;
                                    if state.closed {
                                        remove = true;
                                    } else {
                                        // Hold the gate lock through the ACK. Closing admission
                                        // can never miss a worker that received spawn authority.
                                        state.active += 1;
                                        client.registered = true;
                                        client.stream.write_all(&[1])?;
                                    }
                                }
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) => {}
                        Err(_) => remove = true,
                    }
                }
            } else {
                let mut byte = [0u8; 1];
                match client.stream.read(&mut byte) {
                    Ok(0) => {
                        let mut state = shared.0.lock().map_err(|_| native_error())?;
                        state.active = state.active.checked_sub(1).ok_or_else(native_error)?;
                        remove = true;
                        shared.1.notify_all();
                    }
                    Ok(_) => return Err(native_error()),
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(error),
                }
            }
            if remove {
                clients.swap_remove(index);
            } else {
                index += 1;
            }
        }
        let state = shared.0.lock().map_err(|_| native_error())?;
        if state.closed && state.active == 0 {
            return Ok(());
        }
        drop(state);
        thread::sleep(Duration::from_millis(10));
    }
}
pub(crate) fn begin(workdir: &Path, cleanup_counter: Arc<AtomicUsize>) -> Result<Lease> {
    let work = DirectoryChain::open(workdir)?;
    let parent = native_temp_directory()?;
    let tree = tempfile::Builder::new()
        .prefix("sh-rc-")
        .tempdir_in(&parent)?;
    let path = tree.path().canonicalize()?;
    if path.parent() != Some(parent.as_path()) {
        return Err(failure());
    }
    let held = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(&path)?;
    // SAFETY: the exact newly created private directory handle, never an original.
    if unsafe { libc::fchmod(held.as_raw_fd(), 0o700) } != 0 {
        return Err(failure());
    }
    let chain = DirectoryChain::open(&path)?;
    let directory = OwnedDirectory {
        tree: Some(tree),
        path: path.clone(),
        chain,
        _held: held,
    };
    let socket = path.join("s");
    if !socket_path_fits(&socket) {
        return Err(failure());
    }
    let listener = UnixListener::bind(&socket)?;
    // SAFETY: descriptor-relative metadata on this owned socket node. A raced
    // symlink is never followed to an original/user file.
    if unsafe {
        libc::fchmodat(
            directory._held.as_raw_fd(),
            c"s".as_ptr(),
            0o600,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(failure());
    }
    listener.set_nonblocking(true)?;
    same_chain(&path, &directory.chain)?;
    same_chain(workdir, &work)?;
    let token = Zeroizing::new(uuid::Uuid::new_v4().simple().to_string());
    let thread_token = Zeroizing::new(token.to_string());
    let shared: Shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
    let thread_shared = shared.clone();
    cleanup_counter.fetch_add(1, Ordering::AcqRel);
    let counter = cleanup_counter.clone();
    let spawned = thread::Builder::new()
        .name("rclone-completion".into())
        .spawn(move || {
            let complete = serve(listener, directory, thread_token, &thread_shared).is_ok();
            if let Ok(mut state) = thread_shared.0.lock() {
                state.complete = Some(complete);
                if complete {
                    counter.fetch_sub(1, Ordering::AcqRel);
                }
                thread_shared.1.notify_all();
            }
        });
    let handle = match spawned {
        Ok(handle) => handle,
        Err(error) => {
            cleanup_counter.fetch_sub(1, Ordering::AcqRel);
            return Err(error.into());
        }
    };
    Ok(Lease {
        path: socket,
        token,
        shared,
        thread: Some(handle),
    })
}
pub(crate) fn register() -> Result<UnixStream> {
    let path = PathBuf::from(std::env::var_os(PATH_ENV).ok_or_else(failure)?);
    let token = Zeroizing::new(std::env::var(TOKEN_ENV).map_err(|_| failure())?);
    register_at(&path, &token)
}
fn register_at(path: &Path, token: &str) -> Result<UnixStream> {
    if token.len() != 32 || !token.bytes().all(|b| b.is_ascii_hexdigit()) || !path.is_absolute() {
        return Err(failure());
    }
    let parent = path.parent().ok_or_else(failure)?;
    if !socket_path_fits(path) || parent.parent() != Some(native_temp_directory()?.as_path()) {
        return Err(failure());
    }
    let chain = DirectoryChain::open(parent)?;
    let directory = std::fs::symlink_metadata(parent)?;
    let before = std::fs::symlink_metadata(path)?;
    // SAFETY: read-only identity query.
    let uid = unsafe { libc::geteuid() };
    if !directory.is_dir()
        || directory.uid() != uid
        || directory.mode() & 0o777 != 0o700
        || !before.file_type().is_socket()
        || before.uid() != uid
        || before.mode() & 0o777 != 0o600
    {
        return Err(failure());
    }
    let mut stream = UnixStream::connect(path)?;
    peer(&stream).map_err(|_| failure())?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    same_chain(parent, &chain)?;
    let after = std::fs::symlink_metadata(path)?;
    if before.dev() != after.dev() || before.ino() != after.ino() {
        return Err(failure());
    }
    stream.write_all(token.as_bytes())?;
    let mut ack = [0u8; 1];
    stream.read_exact(&mut ack)?;
    if ack != [1] {
        return Err(failure());
    }
    stream.set_read_timeout(None)?;
    stream.set_write_timeout(None)?;
    Ok(stream)
}

#[cfg(test)]
mod tests;

use super::*;

#[cfg(feature = "fixture-probes")]
struct ProbeOwner(Child);
#[cfg(feature = "fixture-probes")]
impl std::ops::Deref for ProbeOwner {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}
#[cfg(feature = "fixture-probes")]
impl std::ops::DerefMut for ProbeOwner {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}
#[cfg(feature = "fixture-probes")]
impl Drop for ProbeOwner {
    fn drop(&mut self) {
        // Native WNOWAIT reserves our unreaped direct process. Killing the
        // generated owner triggers all native parent-death guardians. Unknown
        // fixture state is retained, never recursively cleaned on failure.
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let mut info = MaybeUninit::<libc::siginfo_t>::zeroed();
            // SAFETY: reserve the exact direct child even on a failed test;
            // an unexpected ECHILD does not authorize killing a reused PID.
            if unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.0.id() as libc::id_t,
                    info.as_mut_ptr(),
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            } != 0
            {
                return;
            }
            let _ = self.0.kill();
            loop {
                match self.0.wait() {
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    _ => break,
                }
            }
        }
    }
}
#[cfg(feature = "fixture-probes")]
fn bounded_probe_wait(owner: &mut ProbeOwner) -> std::process::ExitStatus {
    let until = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(status) = owner.try_wait().unwrap() {
            return status;
        }
        assert!(
            std::time::Instant::now() < until,
            "generated probe exceeded bounded native wait"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn stdio_worker_accepts_only_fixed_flags_and_exact_backend_scope() {
    let scope = OsStr::new("profile:generated/namespace-unique");
    let mut args: Vec<OsString> = FLAGS.into_iter().map(Into::into).collect();
    args.push(scope.into());
    assert!(valid_args(&args, scope));
    for index in 0..FLAGS.len() {
        let mut wrong = args.clone();
        wrong[index] = "--rc".into();
        assert!(!valid_args(&wrong, scope));
    }
    let mut wrong = args.clone();
    wrong.push("--daemon".into());
    assert!(!valid_args(&wrong, scope));
    assert!(!valid_args(&args, OsStr::new("profile:another-scope")));
    assert!(!valid_args(&args, OsStr::new("/unapproved/local/path")));
    assert!(!valid_args(&args, OsStr::new("profile:bad\npath")));
}

#[test]
fn native_lineage_rejects_current_process_as_a_pinned_restic_parent() {
    let root = tempfile::Builder::new()
        .prefix("stillharbour-worker-authorization-")
        .tempdir()
        .unwrap();
    assert!(
        root.path()
            .canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap())
    );
    assert!(authorize(root.path().to_path_buf(), std::process::id() as libc::pid_t).is_err());
    assert!(process(0).is_err());
    assert!(process(1).is_err());
    // A read-only own-process observation is not authority to execute helpers.
    let current = process(std::process::id() as libc::pid_t).unwrap();
    assert_eq!(current.pid, std::process::id() as libc::pid_t);
}

#[test]
#[cfg(feature = "fixture-probes")]
#[ignore = "requires built backup_application_probe and verified Mac ARM64 restic/rclone resources"]
fn actual_pinned_restic_rclone_stop_after_forced_application_owner_death() {
    use std::{
        io::{BufRead, BufReader, Read},
        sync::mpsc,
        time::Instant,
    };
    let executable = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/examples/backup_application_probe")
        .canonicalize()
        .expect("build the actual Mac application probe first");
    let mut owner = ProbeOwner(
        Command::new(executable)
            .arg(super::probe::PROBE_ROLE)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let output = owner.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = BufReader::new(output)
            .take(65537)
            .read_until(b'\n', &mut bytes);
        let _ = sender.send((read, bytes));
    });
    let (read, bytes) = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("actual pinned helper readiness was not observed");
    read.unwrap();
    assert!(bytes.len() <= 65536);
    let ready: serde_json::Value =
        serde_json::from_slice(&bytes).expect("probe must report actual native helper readiness");
    assert_eq!(ready["ready"], true);
    let fixture = PathBuf::from(ready["fixture"].as_str().unwrap())
        .canonicalize()
        .unwrap();
    let marker = ready["owner"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(marker).is_ok());
    assert!(fixture.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        fixture
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!("stillharbour-rclone-guard-{marker}-"))
    );
    assert_eq!(
        std::fs::read(fixture.join("owner")).unwrap(),
        marker.as_bytes()
    );
    let processes: Vec<Process> = serde_json::from_value(ready["processes"].clone()).unwrap();
    assert!(processes.len() >= 5);
    assert!(
        processes
            .iter()
            .all(|p| process(p.pid).ok().as_ref() == Some(p))
    );
    // SAFETY: this exact live direct child remains reserved by our Child handle;
    // no imported process IDs are ever used as signal authority.
    assert_eq!(
        unsafe { libc::kill(owner.id() as libc::pid_t, libc::SIGKILL) },
        0
    );
    assert!(!bounded_probe_wait(&mut owner).success());
    reader.join().unwrap();
    let until = Instant::now() + Duration::from_secs(15);
    while processes
        .iter()
        .any(|p| process(p.pid).ok().as_ref() == Some(p))
    {
        assert!(
            Instant::now() < until,
            "an actual pinned helper survived forced owner death; generated fixture retained"
        );
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        blake3::hash(&std::fs::read(fixture.join("generated.payload")).unwrap())
            .to_hex()
            .as_str(),
        ready["sourceDigest"].as_str().unwrap()
    );
    assert_eq!(
        std::fs::read(fixture.join("neighbor")).unwrap(),
        b"generated untouched neighbor"
    );
    // Only after every recorded native incarnation stopped and boundaries were
    // asserted may this generated-only fixture be recursively cleaned up.
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
#[cfg(feature = "fixture-probes")]
#[ignore = "requires built backup_application_probe and verified Mac ARM64 restic/rclone resources"]
fn actual_pinned_restic_rclone_cancel_and_wrapper_sigkill_finish_before_idle() {
    let executable = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/examples/backup_application_probe")
        .canonicalize()
        .expect("build the actual Mac application probe first");
    for mode in ["cancel", "wrapper-kill"] {
        let mut owner = ProbeOwner(
            Command::new(&executable)
                .args([super::probe::PROBE_ROLE, mode])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut output = owner.stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader = thread::spawn(move || {
            use std::io::Read;
            let mut bytes = Vec::new();
            let read = output.by_ref().take(65537).read_to_end(&mut bytes);
            let _ = sender.send((read, bytes));
        });
        let (read, bytes) = receiver
            .recv_timeout(Duration::from_secs(120))
            .expect("native lifecycle probe exceeded its output deadline");
        read.unwrap();
        assert!(
            bounded_probe_wait(&mut owner).success(),
            "native lifecycle probe failed in {mode}"
        );
        reader.join().unwrap();
        assert!(bytes.len() < 65536);
        let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result["completed"], true);
        assert_eq!(result["mode"], mode);
        assert_eq!(result["sourceRetained"], true);
        assert_eq!(result["neighborRetained"], true);
    }
}

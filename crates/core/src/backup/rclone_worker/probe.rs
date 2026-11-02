//! Generated native lifecycle evidence only; no caller paths or secrets.
use super::*;
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};
pub(super) const PROBE_ROLE: &str = "--stillharbour-rclone-guard-probe-v1";
struct RunningProbe {
    cancel: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<crate::domain::Result<super::super::helpers::HelperOutput>>>,
}
impl RunningProbe {
    fn finish(&mut self) -> io::Result<crate::domain::Result<super::super::helpers::HelperOutput>> {
        self.handle
            .take()
            .ok_or_else(denied)?
            .join()
            .map_err(|_| denied())
    }
}
impl Drop for RunningProbe {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
const FAULT_ROOT: &str = "STILLHARBOUR_RCLONE_PROBE_ROOT";
const FAULT_OWNER: &str = "STILLHARBOUR_RCLONE_PROBE_OWNER";
pub(super) struct WrapperFault(Option<PathBuf>);
impl WrapperFault {
    pub(super) fn arm() -> io::Result<Self> {
        let Some(root) = std::env::var_os(FAULT_ROOT) else {
            return Ok(Self(None));
        };
        let root = PathBuf::from(root).canonicalize()?;
        let owner = std::env::var(FAULT_OWNER).map_err(|_| denied())?;
        if uuid::Uuid::parse_str(&owner).is_err()
            || !root.starts_with(std::env::temp_dir().canonicalize()?)
            || !root
                .file_name()
                .ok_or_else(denied)?
                .to_string_lossy()
                .starts_with(&format!("stillharbour-rclone-guard-{owner}-"))
            || std::fs::read(root.join("owner"))? != owner.as_bytes()
        {
            return Err(denied());
        }
        Ok(Self(Some(root)))
    }
    pub(super) fn fire(&self) {
        if self
            .0
            .as_ref()
            .is_some_and(|root| root.join("kill-wrapper").exists())
        {
            // SAFETY: generated-only fault kills exactly this current wrapper, never
            // an imported/recycled PID. Its C guardian must retain completion proof.
            unsafe {
                libc::kill(libc::getpid(), libc::SIGKILL);
            }
        }
    }
}

pub(super) fn descendants(owner: libc::pid_t) -> io::Result<Vec<Process>> {
    let mut pending = VecDeque::from([(owner, 0usize)]);
    let mut found = Vec::new();
    while let Some((parent, depth)) = pending.pop_front() {
        if depth >= 6 {
            return Err(denied());
        }
        let mut pids = [0 as libc::pid_t; 32];
        // SAFETY: bounded output, read-only native parent relationship query.
        let count = unsafe {
            libc::proc_listchildpids(
                parent,
                pids.as_mut_ptr().cast(),
                size_of::<[libc::pid_t; 32]>() as i32,
            )
        };
        // libproc's convenience wrapper returns PID COUNT (proc_listpids
        // returns bytes). Saturation cannot certify a complete observation.
        if count < 0 || count as usize >= pids.len() {
            return Err(denied());
        }
        for pid in pids.into_iter().take(count as usize) {
            let Ok(child) = process(pid) else {
                continue;
            };
            if child.parent != parent {
                return Err(denied());
            }
            pending.push_back((pid, depth + 1));
            found.push(child);
            if found.len() > 32 {
                return Err(denied());
            }
        }
    }
    Ok(found)
}
fn run_probe(mode: &str) -> io::Result<i32> {
    let owner = uuid::Uuid::new_v4().to_string();
    let fixture = tempfile::Builder::new()
        .prefix(&format!("stillharbour-rclone-guard-{owner}-"))
        .tempdir()?;
    let root = fixture.path().canonicalize()?;
    if !root.starts_with(std::env::temp_dir().canonicalize()?) {
        return Err(denied());
    }
    std::fs::write(root.join("owner"), owner.as_bytes())?;
    std::fs::write(root.join("neighbor"), b"generated untouched neighbor")?;
    let helper_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/backup-helpers/macos-aarch64")
        .canonicalize()?;
    let helpers = convert(PinnedHelpers::configure(helper_root.clone()))?;
    let (_, rclone, _) = convert(helpers.verified(Helper::Rclone))?;
    let rclone_id = convert(filesystem::identity(&rclone))?;
    let (_, restic, _) = convert(helpers.verified(Helper::Restic))?;
    let restic_id = convert(filesystem::identity(&restic))?;
    let source = root.join("generated.payload");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&source)?;
    let mut random = File::open("/dev/urandom")?;
    let mut chunk = [0u8; 65536];
    for _ in 0..256 {
        random.read_exact(&mut chunk)?;
        file.write_all(&chunk)?;
    }
    file.sync_all()?;
    drop(file);
    let expected_source = blake3::hash(&std::fs::read(&source)?);
    let config = root.join("generated.conf");
    std::fs::write(&config, b"[profile]\ntype = local\n")?;
    let scope = format!(
        "profile:{}",
        root.join("repository").to_str().ok_or_else(denied)?
    );
    let mut environment: BTreeMap<OsString, OsString> = BTreeMap::new();
    for (name, value) in [
        (ROOT_ENV, helper_root.into_os_string()),
        (OWNER_ENV, std::process::id().to_string().into()),
        (SCOPE_ENV, scope.clone().into()),
        ("RESTIC_PASSWORD", uuid::Uuid::new_v4().to_string().into()),
        ("RCLONE_CONFIG", config.into_os_string()),
        ("RCLONE_BWLIMIT", "10M".into()),
        ("GOMAXPROCS", "2".into()),
    ] {
        environment.insert(name.into(), value);
    }
    let base: Vec<OsString> = vec![
        "--repo".into(),
        format!("rclone:{scope}").into(),
        "--no-cache".into(),
        "-o".into(),
        format!("rclone.program={}", convert(helpers.rclone_program())?).into(),
        "-o".into(),
        format!("rclone.args={}", FLAGS.join(" ")).into(),
        "-o".into(),
        "rclone.connections=1".into(),
    ];
    let cancel = Arc::new(AtomicBool::new(false));
    let mut init = base.clone();
    init.push("init".into());
    let initialized = convert(helpers.run(
        Helper::Restic,
        &init,
        &environment,
        &root,
        &cancel,
        Duration::from_secs(90),
    ))?;
    if initialized.code != 0 {
        return Err(denied());
    }
    let mut backup = base;
    backup.extend([
        "--limit-upload".into(),
        "64".into(),
        "backup".into(),
        "--compression".into(),
        "off".into(),
        source.clone().into_os_string(),
    ]);
    environment.insert("RCLONE_BWLIMIT".into(), "64k".into());
    if mode == "wrapper-kill" {
        environment.insert(FAULT_ROOT.into(), root.clone().into_os_string());
        environment.insert(FAULT_OWNER.into(), owner.clone().into());
    }
    let worker_helpers = helpers.clone();
    let work = root.clone();
    let worker_cancel = cancel.clone();
    let handle = thread::spawn(move || {
        worker_helpers.run(
            Helper::Restic,
            &backup,
            &environment,
            &work,
            &worker_cancel,
            Duration::from_secs(120),
        )
    });
    // This guard is dropped before the TempDir even on an inspection failure.
    let mut running = RunningProbe {
        cancel: cancel.clone(),
        handle: Some(handle),
    };
    let until = Instant::now() + Duration::from_secs(45);
    let observed = loop {
        let current = descendants(std::process::id() as libc::pid_t)?;
        if current.iter().any(|p| same_image(&p.image, &rclone_id))
            && current.iter().any(|p| same_image(&p.image, &restic_id))
        {
            break current;
        }
        if running
            .handle
            .as_ref()
            .is_none_or(|handle| handle.is_finished())
            || Instant::now() >= until
        {
            cancel.store(true, std::sync::atomic::Ordering::Release);
            let _ = running.finish();
            return Err(denied());
        }
        thread::sleep(Duration::from_millis(50));
    };
    if mode != "owner-kill" {
        if mode == "wrapper-kill" {
            std::fs::write(root.join("kill-wrapper"), b"generated self-kill fault")?;
        } else {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
        let result = running.finish()?;
        if result.is_ok_and(|result| result.code == 0) {
            return Err(denied());
        }
        // Completion is required when run() returns, not after a fixed grace
        // sleep. Every observed native incarnation must already be stopped.
        if observed
            .iter()
            .any(|p| process(p.pid).ok().as_ref() == Some(p))
        {
            return Err(denied());
        }
        let source_retained = blake3::hash(&std::fs::read(&source)?) == expected_source;
        println!(
            "{}",
            serde_json::json!({"completed":true,"mode":mode,"sourceRetained":source_retained,"neighborRetained":std::fs::read(root.join("neighbor"))?==b"generated untouched neighbor"})
        );
        return Ok(0);
    }
    // Only generated fixture paths/native process incarnations cross this probe
    // output. They never become production execution or signal authority.
    let source_digest = blake3::hash(&std::fs::read(root.join("generated.payload"))?)
        .to_hex()
        .to_string();
    println!(
        "{}",
        serde_json::json!({"ready":true,"owner":owner,"fixture":root,"processes":observed,"sourceDigest":source_digest})
    );
    io::stdout().flush()?;
    // The external test owns this process and deliberately SIGKILLs it. On an
    // ordinary run, retain the bounded helper timeout and normal TempDir cleanup.
    let outcome = running.finish()?;
    Ok(if convert(outcome)?.code == 0 { 0 } else { 125 })
}
pub fn guard_probe_dispatch() -> Option<i32> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(OsStr::new(PROBE_ROLE)) {
        return None;
    }
    let mode = args.next().unwrap_or_else(|| "owner-kill".into());
    let Some(mode) = mode
        .to_str()
        .filter(|mode| ["owner-kill", "wrapper-kill", "cancel"].contains(mode))
    else {
        return Some(126);
    };
    if args.next().is_some() {
        return Some(126);
    }
    Some(run_probe(mode).unwrap_or(126))
}

use super::*;
use std::{
    path::PathBuf,
    process::{Child, Stdio},
    time::{Duration, Instant},
};

fn fixture() -> Option<PathBuf> {
    let root = std::env::var_os("STILLHARBOUR_GUARD_FIXTURE")?;
    let root = PathBuf::from(root).canonicalize().unwrap();
    let owner = std::env::var("STILLHARBOUR_GUARD_OWNER").unwrap();
    assert!(uuid::Uuid::parse_str(&owner).is_ok());
    assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!("stillharbour-helper-guard-{owner}-"))
    );
    assert_eq!(std::fs::read(root.join("owner")).unwrap(), owner.as_bytes());
    Some(root)
}
fn command(root: &std::path::Path, owner: &str, role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", role, "--nocapture"])
        .env("STILLHARBOUR_GUARD_FIXTURE", root)
        .env("STILLHARBOUR_GUARD_OWNER", owner)
        .env_remove("STILLHARBOUR_GUARD_EARLY_EXIT")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
fn bytes(root: &std::path::Path) -> u64 {
    std::fs::metadata(root.join("heartbeat"))
        .map(|m| m.len())
        .unwrap_or(0)
}
fn wait_for(mut condition: impl FnMut() -> bool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert!(Instant::now() < deadline, "{message}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn wait_exit(child: &mut Child) -> std::process::ExitStatus {
    let mut status = None;
    wait_for(
        || {
            status = child.try_wait().unwrap();
            status.is_some()
        },
        "Owned helper supervisor did not exit/reap.",
    );
    status.unwrap()
}
fn no_heartbeat(root: &std::path::Path) {
    let mut previous = bytes(root);
    let mut stable = 0;
    wait_for(
        || {
            std::thread::sleep(Duration::from_millis(100));
            let current = bytes(root);
            if current == previous {
                stable += 1;
            } else {
                stable = 0;
            }
            previous = current;
            stable >= 5
        },
        "An owned helper descendant kept writing after cancellation/parent death.",
    );
    let final_bytes = bytes(root);
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(bytes(root), final_bytes);
}
fn owned() -> (tempfile::TempDir, String) {
    let owner = uuid::Uuid::new_v4().to_string();
    let root = tempfile::Builder::new()
        .prefix(&format!("stillharbour-helper-guard-{owner}-"))
        .tempdir()
        .unwrap();
    std::fs::write(root.path().join("owner"), owner.as_bytes()).unwrap();
    (root, owner)
}

#[test]
fn helper_process() {
    let Some(root) = fixture() else {
        return;
    };
    std::fs::write(root.join("helper_pid"), std::process::id().to_string()).unwrap();
    let mut child = Command::new("/bin/sh")
        .args([
            "-c",
            "while :; do printf x >> \"$STILLHARBOUR_GUARD_HEARTBEAT\"; /bin/sleep 0.05; done",
        ])
        .env("STILLHARBOUR_GUARD_HEARTBEAT", root.join("heartbeat"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::fs::write(root.join("descendant_pid"), child.id().to_string()).unwrap();
    wait_for(
        || bytes(&root) > 0,
        "Generated helper heartbeat never started.",
    );
    if std::env::var_os("STILLHARBOUR_GUARD_EARLY_EXIT").is_some() {
        // Deliberately leave this generated descendant for the C supervisor's
        // group cleanup when the actual helper exits normally.
        std::mem::forget(child);
        return;
    }
    loop {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(50));
    }
}
#[test]
fn parent_process() {
    let Some(root) = fixture() else {
        return;
    };
    let owner = std::env::var("STILLHARBOUR_GUARD_OWNER").unwrap();
    let mut command = command(&root, &owner, "backup::helper_guard::tests::helper_process");
    configure(&mut command).unwrap();
    let mut child = command.spawn().unwrap();
    std::fs::write(root.join("supervisor_pid"), child.id().to_string()).unwrap();
    loop {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(50));
    }
}
#[test]
fn mac_helper_guard_hard_parent_death_stops_actual_descendant_heartbeat() {
    let (root, owner) = owned();
    let mut parent = command(
        root.path(),
        &owner,
        "backup::helper_guard::tests::parent_process",
    )
    .spawn()
    .unwrap();
    wait_for(
        || bytes(root.path()) > 1 && root.path().join("supervisor_pid").exists(),
        "Generated guarded app/helper did not start.",
    );
    // SAFETY: this exact live direct child is owned by this test and remains
    // waitable; no process-name or unverified PID/group cleanup is used.
    assert_eq!(
        unsafe { libc::kill(parent.id() as libc::pid_t, libc::SIGKILL) },
        0
    );
    assert!(!wait_exit(&mut parent).success());
    no_heartbeat(root.path());
}
#[test]
fn mac_helper_guard_cancel_and_natural_exit_reap_owned_helper_group() {
    for early in [false, true] {
        let (root, owner) = owned();
        let mut command = command(
            root.path(),
            &owner,
            "backup::helper_guard::tests::helper_process",
        );
        if early {
            command.env("STILLHARBOUR_GUARD_EARLY_EXIT", "1");
        }
        configure(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        wait_for(
            || bytes(root.path()) > 0,
            "Generated helper descendant did not start.",
        );
        if !early {
            // SAFETY: exact live owned supervisor Child; normal cancellation is SIGTERM.
            assert_eq!(
                unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) },
                0
            );
        }
        let status = wait_exit(&mut child);
        assert_eq!(status.success(), early);
        no_heartbeat(root.path());
    }
}

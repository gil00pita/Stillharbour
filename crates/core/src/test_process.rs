//! Owned isolation for native workflow liveness scenarios. Fault matrices in
//! the parent deliberately share/saturate a paced-I/O budget; the desktop has
//! its own process budget. No path or adoption authority crosses this boundary.
pub(crate) fn isolated(case: &str, seconds: u64) -> bool {
    assert!((1..=120).contains(&seconds));
    const CASE: &str = "STILLHARBOUR_ISOLATED_TEST_CASE";
    const TOKEN: &str = "STILLHARBOUR_ISOLATED_TEST_TOKEN";
    if let Some(selected) = std::env::var_os(CASE) {
        assert_eq!(selected.to_str(), Some(case));
        uuid::Uuid::parse_str(&std::env::var(TOKEN).unwrap()).unwrap();
        return false;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", case, "--nocapture"])
        .env(CASE, case)
        .env(TOKEN, uuid::Uuid::new_v4().to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Owned scenario exceeded {seconds} seconds: {case}");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(
        result.status.success(),
        "{} {}",
        output,
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        output.contains("running 1 test") && output.contains("test result: ok. 1 passed;"),
        "Owned isolation must execute its selected case, not silently match zero tests: {case}: {output}"
    );
    true
}

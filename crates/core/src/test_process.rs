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
    // Drain both pipes while the child runs. Waiting for process exit before
    // reading can deadlock on Windows' bounded anonymous-pipe buffers.
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let mut timed_out = false;
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    let output = stdout.finish();
    let errors = stderr.finish();
    assert!(
        !timed_out,
        "Owned scenario exceeded {seconds} seconds: {case}\n{output}\n{errors}"
    );
    assert!(status.success(), "{} {}", output, errors);
    assert!(
        output.contains("running 1 test") && output.contains("test result: ok. 1 passed;"),
        "Owned isolation must execute its selected case, not silently match zero tests: {case}: {output}"
    );
    true
}

struct Output {
    tail: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    done: std::sync::mpsc::Receiver<()>,
}
impl Output {
    fn finish(self) -> String {
        // A descendant retaining an inherited writer must not remove the
        // parent's deadline. Keep the captured tail even if EOF is delayed.
        let _ = self.done.recv_timeout(std::time::Duration::from_secs(1));
        String::from_utf8_lossy(&self.tail.lock().unwrap()).into_owned()
    }
}
fn drain(mut stream: impl std::io::Read + Send + 'static) -> Output {
    let tail = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = tail.clone();
    let (sender, done) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut chunk = [0; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    let mut bytes = captured.lock().unwrap();
                    bytes.extend_from_slice(&chunk[..count]);
                    let excess = bytes.len().saturating_sub(64 * 1024);
                    // Keep the initial test-count proof and the latest error.
                    if excess > 0 {
                        bytes.drain(4096..4096 + excess);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = sender.send(());
    });
    Output { tail, done }
}

#[test]
fn isolated_output_drains_more_than_a_pipe_buffer_and_bounds_retained_diagnostics() {
    let bytes = vec![b'x'; 256 * 1024];
    let output = drain(std::io::Cursor::new(bytes));
    let captured = output.finish();
    assert_eq!(captured.len(), 64 * 1024);
    assert!(captured.bytes().all(|byte| byte == b'x'));
}

#[test]
fn isolated_child_with_output_larger_than_both_pipe_buffers_completes() {
    if isolated(
        "test_process::isolated_child_with_output_larger_than_both_pipe_buffers_completes",
        10,
    ) {
        return;
    }
    use std::io::Write;
    for _ in 0..64 {
        std::io::stdout().write_all(&[b'o'; 4096]).unwrap();
        std::io::stderr().write_all(&[b'e'; 4096]).unwrap();
    }
}

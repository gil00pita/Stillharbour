//! One pinned argument-vector policy for the backend and macOS private worker.
//! Timeouts bound stalled I/O; healthy progressing transfers have no short cap.
pub(crate) const RCLONE_STDIO_ARGS: [&str; 11] = [
    "serve",
    "restic",
    "--stdio",
    "--transfers=1",
    "--checkers=1",
    "--buffer-size=64k",
    "--multi-thread-streams=0",
    "--contimeout=10s",
    "--timeout=30s",
    "--retries=1",
    "--low-level-retries=2",
];
pub(crate) const RESTIC_RCLONE_STARTUP: &str = "20s";
pub(crate) const RESTIC_STUCK_REQUEST: &str = "30s";

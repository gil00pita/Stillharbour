use std::{path::Path, time::Duration};
use stillharbour_core::{domain::ErrorCode, media::MediaWorker};

#[tokio::test]
async fn malformed_worker_is_replaced_and_originals_survive() {
    let temp = tempfile::tempdir().unwrap();
    // Windows forbids quotes/control characters in filesystem names; retain the
    // hostile Unix name there and a valid Unicode/leading-hyphen case on Windows.
    let name = if cfg!(windows) {
        "-broken%港📷.jpg"
    } else {
        "-broken\"%\n港📷.jpg"
    };
    let broken = temp.path().join(name);
    std::fs::write(&broken, b"not a valid image").unwrap();
    let valid = temp.path().join("generated.png");
    image::RgbImage::from_pixel(64, 32, image::Rgb([20, 80, 50]))
        .save(&valid)
        .unwrap();
    let bytes = std::fs::read(&valid).unwrap();
    let worker = MediaWorker::new(Duration::from_secs(5)).unwrap();
    let executable = Path::new(env!("CARGO_BIN_EXE_stillharbour-media-worker"));
    assert_eq!(
        worker
            .thumbnail(executable, &broken, temp.path())
            .await
            .unwrap_err()
            .code,
        ErrorCode::WorkerFailed
    );
    let report = worker
        .thumbnail(executable, &valid, temp.path())
        .await
        .unwrap();
    assert_eq!((report.thumbnail_width, report.thumbnail_height), (64, 32));
    assert_eq!(std::fs::read(&valid).unwrap(), bytes);
    assert_eq!(std::fs::read(&broken).unwrap(), b"not a valid image");
}

#[tokio::test]
#[cfg(unix)]
async fn stalled_child_is_killed_and_next_item_can_run() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let temp = tempfile::tempdir().unwrap();
    let fifo = temp.path().join("owned-stall");
    let native = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: this FIFO is created only inside this test's fresh TempDir.
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    let worker = MediaWorker::new(Duration::from_secs(2)).unwrap();
    let executable = Path::new(env!("CARGO_BIN_EXE_stillharbour-media-worker"));
    assert_eq!(
        worker
            .thumbnail(executable, &fifo, temp.path())
            .await
            .unwrap_err()
            .code,
        ErrorCode::WorkerTimeout
    );
    let valid = temp.path().join("next.png");
    image::RgbImage::from_pixel(16, 16, image::Rgb([0, 0, 0]))
        .save(&valid)
        .unwrap();
    assert!(
        worker
            .thumbnail(executable, &valid, temp.path())
            .await
            .is_ok()
    );
}

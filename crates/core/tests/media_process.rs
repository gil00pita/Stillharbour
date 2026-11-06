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

#[tokio::test]
async fn stdin_metadata_worker_reads_generated_bytes_and_rejects_path_arguments() {
    use tokio::io::AsyncWriteExt;
    async fn invoke(bytes: &[u8], extra: bool) -> std::process::Output {
        let mut command =
            tokio::process::Command::new(env!("CARGO_BIN_EXE_stillharbour-media-worker"));
        command.arg("--metadata-worker-stdin-v1");
        if extra {
            command.arg("--unexpected-input-path");
        }
        let mut child = command
            .env_clear()
            .kill_on_drop(true)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let written = input.write_all(bytes).await;
        drop(input);
        if !extra {
            written.unwrap();
        }
        tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
            .await
            .unwrap()
            .unwrap()
    }
    let owned = tempfile::Builder::new()
        .prefix("stillharbour-metadata-cli-")
        .tempdir()
        .unwrap();
    let path = owned.path().join("generated.png");
    image::RgbImage::from_pixel(64, 32, image::Rgb([20, 80, 50]))
        .save(&path)
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    let output = invoke(&original, false).await;
    assert!(
        output.status.success(),
        "metadata worker status: {:?}",
        output.status
    );
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data[0]["ImageWidth"], 64);
    assert_eq!(data[0]["ImageHeight"], 32);
    assert_eq!(data[0]["StillharbourMetadataPolicy"], "windows-exif-v3");
    assert!(data[0].get("GPSLatitude").is_none());
    let rejected = invoke(&original, true).await;
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    #[cfg(feature = "fixture-probes")]
    {
        let fixture =
            stillharbour_core::library_fixture::LibraryFixture::generate_geographic().unwrap();
        let bytes = std::fs::read(fixture.path().join("West.jpg")).unwrap();
        let output = invoke(&bytes, false).await;
        assert!(output.status.success());
        let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(data[0]["GPSLatitude"], 5.0);
        assert!((data[0]["GPSLongitude"].as_f64().unwrap() + 179.75).abs() < 0.00001);
        assert_eq!(fixture.verify_unchanged().unwrap(), 9);
        let tagged = stillharbour_core::library_fixture::LibraryFixture::generate_tagged().unwrap();
        let count = tagged.verify_unchanged().unwrap();
        let bytes = std::fs::read(tagged.path().join("Coast/Generated-00.jpg")).unwrap();
        let output = invoke(&bytes, false).await;
        assert!(output.status.success());
        let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            data[0]["Keywords"],
            serde_json::json!(["Coast", "Outdoors"])
        );
        assert_eq!(tagged.verify_unchanged().unwrap(), count);
    }
}

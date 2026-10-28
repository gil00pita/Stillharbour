//! Read-only helper supervision. Inputs are bounded bytes from authorized handles,
//! never renderer paths or a line-based filename protocol.
use super::{filters::invalid, store::Metadata};
use crate::{
    domain::{AppError, ErrorCode, Result},
    media::{MAX_INPUT_BYTES, MediaReport},
};
use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Clone)]
pub struct WorkerPaths {
    pub executable: PathBuf,
    pub exiftool: Option<PathBuf>,
    pub working_directory: PathBuf,
}

pub fn read_input(file: File) -> Result<Vec<u8>> {
    read_input_at(file, crate::performance::Endpoint::Source)
}
pub fn read_input_at(mut file: File, endpoint: crate::performance::Endpoint) -> Result<Vec<u8>> {
    let started = crate::performance::clock();
    let volume = crate::performance::file_volume(&file);
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_INPUT_BYTES {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "This file exceeds the current 32 MiB preview/metadata limit.",
        ));
    }
    let before = super::native::revision(&file)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT_BYTES || super::native::revision(&file)? != before {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "The original changed during reading. Rescan it.",
        ));
    }
    if let Some(started) = started {
        crate::performance::record_at(
            crate::performance::Metric::Read,
            endpoint,
            volume,
            started.elapsed().as_micros().min(u64::MAX as u128) as u64,
            bytes.len() as u64,
        );
    }
    Ok(bytes)
}

async fn run(
    mut command: Command,
    input: Vec<u8>,
    limit: u64,
    monitor_decoder: bool,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<u8>> {
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| invalid("Missing worker identity."))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| invalid("Missing worker input."))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("Missing worker output."))?;
    let processing = async {
        let writing = async move {
            for chunk in input.chunks(65536) {
                stdin.write_all(chunk).await?;
            }
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, std::io::Error>(())
        };
        let reading = async {
            let mut bytes = Vec::new();
            (&mut stdout)
                .take(limit + 1)
                .read_to_end(&mut bytes)
                .await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let (_, bytes) = tokio::try_join!(writing, reading)?;
        if bytes.len() as u64 > limit {
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "The metadata/preview output exceeded its budget.",
            ));
        }
        if !child.wait().await?.success() {
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "The metadata/preview worker could not read this file.",
            ));
        }
        Ok(bytes)
    };
    let cancellation = async {
        loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    };
    let result = tokio::select! {
        result=tokio::time::timeout(Duration::from_secs(12),processing)=>result.unwrap_or_else(|_|Err(AppError::new(ErrorCode::WorkerTimeout,"The metadata/preview worker timed out."))),
        error=crate::media::watch_decoder(pid), if monitor_decoder=>Err(error),
        _=cancellation=>Err(AppError::new(ErrorCode::Busy,"The read job was canceled.")),
    };
    if result.is_err() {
        let _ = child.kill().await;
    }
    result
}

pub async fn metadata(
    paths: &WorkerPaths,
    bytes: Vec<u8>,
    cancel: Arc<AtomicBool>,
) -> Result<Metadata> {
    let script = paths.exiftool.as_ref().ok_or_else(|| {
        AppError::new(
            ErrorCode::Unsupported,
            "The packaged metadata helper is unavailable on this platform.",
        )
    })?;
    let library = script
        .parent()
        .ok_or_else(|| invalid("Invalid metadata helper layout."))?
        .join("lib");
    let mut command = Command::new("/usr/bin/perl");
    let mut include = std::ffi::OsString::from("-I");
    include.push(library);
    command
        .arg(include)
        .arg(script)
        .args(crate::backup::metadata_stdin_arguments())
        .env_clear()
        .current_dir(&paths.working_directory);
    let digest = blake3::hash(&bytes).to_hex().to_string();
    let output = run(command, bytes, 1024 * 1024, false, cancel).await?;
    let raw: String =
        String::from_utf8(output).map_err(|_| invalid("Metadata output is not UTF-8."))?;
    let values: Vec<Value> = serde_json::from_str(&raw)
        .map_err(|e| invalid("Metadata output is invalid.").caused_by(e))?;
    let value = values
        .first()
        .ok_or_else(|| invalid("Metadata output was empty."))?;
    if value.get("Error").is_some() {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "The metadata helper could not interpret this file.",
        ));
    }
    let field = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let captured = capture_date(value);
    let camera = match (field("Make"), field("Model")) {
        (Some(a), Some(b)) => Some(format!("{} {}", a.trim(), b.trim())),
        (a, b) => a.or(b),
    };
    Ok(Metadata {
        date: captured.as_ref().map(|d| d.date.clone()),
        source: captured
            .as_ref()
            .map(|d| d.source)
            .unwrap_or("unavailable")
            .into(),
        raw_date: captured
            .as_ref()
            .map(|d| d.raw.clone())
            .or_else(|| field("DateTimeOriginal").or_else(|| field("CreateDate"))),
        offset: captured.and_then(|d| d.offset),
        width: value
            .get("ImageWidth")
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
            .and_then(|n| u32::try_from(n).ok()),
        height: value
            .get("ImageHeight")
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
            .and_then(|n| u32::try_from(n).ok()),
        camera,
        format: field("FileType").map(|s| s.to_ascii_lowercase()),
        raw,
        digest: Some(digest),
    })
}

struct CaptureDate {
    date: String,
    source: &'static str,
    raw: String,
    offset: Option<String>,
}
fn capture_date(value: &Value) -> Option<CaptureDate> {
    for (tag, subsec, offset, source) in [
        (
            "DateTimeOriginal",
            "SubSecTimeOriginal",
            "OffsetTimeOriginal",
            "embedded_original",
        ),
        (
            "CreateDate",
            "SubSecTimeDigitized",
            "OffsetTimeDigitized",
            "embedded_created",
        ),
    ] {
        let Some(raw) = value.get(tag).and_then(Value::as_str) else {
            continue;
        };
        let Some(date) = normalize_date(raw, value.get(subsec).and_then(Value::as_str)) else {
            continue;
        };
        let inline = raw.get(19..).and_then(|tail| {
            if tail.ends_with('Z') {
                Some("+00:00")
            } else {
                tail.char_indices()
                    .find(|(_, c)| matches!(c, '+' | '-'))
                    .map(|(i, _)| &tail[i..])
            }
        });
        let offset = value
            .get(offset)
            .and_then(Value::as_str)
            .or(inline)
            .filter(|s| s.parse::<chrono::FixedOffset>().is_ok())
            .map(str::to_owned);
        return Some(CaptureDate {
            date,
            source,
            raw: raw.to_owned(),
            offset,
        });
    }
    None
}

pub fn normalize_date(raw: &str, subsec: Option<&str>) -> Option<String> {
    // Keep capture-local wall time. Offset provenance remains separate; unknown
    // offsets are never invented as UTC. Never depend on the machine timezone.
    let base = raw.get(..19)?;
    let parsed = chrono::NaiveDateTime::parse_from_str(base, "%Y:%m:%d %H:%M:%S").ok()?;
    let inline_fraction = raw.get(19..).and_then(|s| s.strip_prefix('.')).map(|s| {
        s.split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or_default()
    });
    let fraction = subsec
        .or(inline_fraction)
        .filter(|s| !s.is_empty() && s.len() <= 9 && s.chars().all(|c| c.is_ascii_digit()))
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    Some(format!("{}{fraction}", parsed.format("%Y-%m-%dT%H:%M:%S")))
}

pub async fn thumbnail(
    paths: &WorkerPaths,
    bytes: Vec<u8>,
    edge: u32,
    format: crate::media::PreviewFormat,
    cancel: Arc<AtomicBool>,
) -> Result<MediaReport> {
    let _span = crate::performance::Span::new(
        crate::performance::Metric::ThumbnailWorker,
        crate::performance::Endpoint::Worker,
    );
    let mut command = Command::new(&paths.executable);
    command
        .arg("--media-worker-stdin")
        .arg(edge.to_string())
        .arg((format as u32).to_string())
        .env_clear()
        .current_dir(&paths.working_directory);
    let output = run(command, bytes, 8 * 1024 * 1024, true, cancel).await?;
    serde_json::from_slice::<Result<MediaReport>>(&output).map_err(|e| {
        AppError::new(
            ErrorCode::WorkerFailed,
            "Preview worker returned invalid data.",
        )
        .caused_by(e)
    })?
}

pub fn helper_available(paths: &WorkerPaths) -> bool {
    cfg!(target_os = "macos")
        && Path::new("/usr/bin/perl").is_file()
        && paths.exiftool.as_ref().is_some_and(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_dates_remain_local_and_preserve_old_and_fractional_values() {
        assert_eq!(
            normalize_date("1964:02:29 13:15:20", Some("1234")).as_deref(),
            Some("1964-02-29T13:15:20.1234")
        );
        assert_eq!(
            normalize_date("2024:03:31 01:30:00", None).as_deref(),
            Some("2024-03-31T01:30:00")
        );
        assert!(normalize_date("2024:02:30 12:00:00", None).is_none());
    }
    #[test]
    fn invalid_original_falls_back_to_created_without_losing_fraction_or_offset() {
        let date=capture_date(&serde_json::json!({"DateTimeOriginal":"not a date","CreateDate":"1964:02:29 13:15:20.0004+05:30"})).unwrap();
        assert_eq!(date.source, "embedded_created");
        assert_eq!(date.date, "1964-02-29T13:15:20.0004");
        assert_eq!(date.offset.as_deref(), Some("+05:30"));
        let unknown =
            capture_date(&serde_json::json!({"DateTimeOriginal":"2024:03:31 01:30:00"})).unwrap();
        assert!(unknown.offset.is_none());
    }
}

//! Short-lived decoder worker. It accepts only backend-owned handles/paths;
//! process isolation and time/output caps are separate from decoder allocation limits.
use crate::{
    domain::{AppError, ErrorCode, Result},
    jobs::JobScheduler,
};
use image::{ImageDecoder, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read},
    path::Path,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

pub const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: u64 = 2 * 1024 * 1024;

/// Internal worker protocol; derived from catalog filename suffixes, not renderer input.
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum PreviewFormat {
    Auto,
    Cr2,
    Cr3,
    Nef,
    Arw,
    Dng,
    Raf,
    Orf,
    Rw2,
    Crw,
    Pef,
}
impl PreviewFormat {
    pub fn from_extension(extension: &str) -> Self {
        if extension.len() > 16 {
            return Self::Auto;
        }
        match extension.to_ascii_lowercase().as_str() {
            "cr2" => Self::Cr2,
            "cr3" => Self::Cr3,
            "nef" => Self::Nef,
            "arw" => Self::Arw,
            "dng" => Self::Dng,
            "raf" => Self::Raf,
            "orf" => Self::Orf,
            "rw2" => Self::Rw2,
            "crw" => Self::Crw,
            "pef" => Self::Pef,
            _ => Self::Auto,
        }
    }
    fn from_code(code: u32) -> Result<Self> {
        Ok(match code {
            0 => Self::Auto,
            1 => Self::Cr2,
            2 => Self::Cr3,
            3 => Self::Nef,
            4 => Self::Arw,
            5 => Self::Dng,
            6 => Self::Raf,
            7 => Self::Orf,
            8 => Self::Rw2,
            9 => Self::Crw,
            10 => Self::Pef,
            _ => {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Unknown preview format hint.",
                ));
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaReport {
    pub source_width: u32,
    pub source_height: u32,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub orientation_applied: bool,
    pub color_policy: String,
    pub png: Vec<u8>,
}

pub fn decode_thumbnail(path: &Path) -> Result<MediaReport> {
    let mut file = crate::filesystem::open_readonly(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_INPUT_BYTES {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Image exceeds the fixture decoder input limit.",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    decode_bytes(
        &bytes,
        256,
        PreviewFormat::from_extension(
            path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default(),
        ),
    )
}

fn decode_bytes(bytes: &[u8], edge: u32, hint: PreviewFormat) -> Result<MediaReport> {
    if bytes.len() as u64 > MAX_INPUT_BYTES || !matches!(edge, 256 | 512) {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Preview exceeds its input or size budget.",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = hint;
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let baseline = matches!(
        reader.format(),
        Some(image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP)
    );
    if baseline {
        let result = decode_reader(reader, edge);
        // Only an explicit unsupported color policy may use the native color
        // transform. Never bypass malformed-image or allocation-limit failures.
        #[cfg(target_os = "macos")]
        if matches!(&result,Err(error) if error.code == ErrorCode::Unsupported) {
            let mut decoder = ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()?
                .into_decoder()
                .map_err(decode_error)?;
            let valid = decoder
                .icc_profile()
                .map_err(decode_error)?
                .as_deref()
                .is_some_and(crate::native_preview::valid_icc);
            if !valid {
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    "The embedded color profile is invalid or exceeds the preview profile budget.",
                ));
            }
            return crate::native_preview::decode(bytes, edge, PreviewFormat::Auto);
        }
        return result;
    }
    #[cfg(target_os = "macos")]
    {
        crate::native_preview::decode(bytes, edge, hint)
    }
    #[cfg(not(target_os = "macos"))]
    {
        decode_reader(reader, edge)
    }
}

fn decode_reader<R: std::io::BufRead + std::io::Seek>(
    mut reader: ImageReader<R>,
    edge: u32,
) -> Result<MediaReport> {
    let decode_start = crate::performance::clock();
    if edge != 256 && edge != 512 {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Preview edge must be 256 or 512 pixels.",
        ));
    }
    if reader.format().is_some_and(|format| {
        !matches!(
            format,
            image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
        )
    }) {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Previews currently support JPEG, PNG and WebP. This original was retained.",
        ));
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    let (width, height) = decoder.dimensions();
    // Do not silently interpret ICC data without a tested transform pipeline.
    if decoder.icc_profile().map_err(decode_error)?.is_some() {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "ICC-managed previews require the later tested color pipeline.",
        ));
    }
    let orientation = decoder.orientation().map_err(decode_error)?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    crate::performance::since(
        crate::performance::Metric::Decode,
        crate::performance::Endpoint::Worker,
        decode_start,
        0,
    );
    let resize_start = crate::performance::clock();
    image.apply_orientation(orientation);
    let thumbnail = if image.width() <= edge && image.height() <= edge {
        image
    } else {
        image.thumbnail(edge, edge)
    };
    crate::performance::since(
        crate::performance::Metric::Resize,
        crate::performance::Endpoint::Worker,
        resize_start,
        0,
    );
    let encode_start = crate::performance::clock();
    let mut output = Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(decode_error)?;
    crate::performance::since(
        crate::performance::Metric::Encode,
        crate::performance::Endpoint::Worker,
        encode_start,
        output.get_ref().len() as u64,
    );
    Ok(MediaReport {
        source_width: width,
        source_height: height,
        thumbnail_width: thumbnail.width(),
        thumbnail_height: thumbnail.height(),
        orientation_applied: orientation != image::metadata::Orientation::NoTransforms,
        color_policy: "Untagged fixture RGB; ICC inputs are refused, not silently converted."
            .into(),
        png: output.into_inner(),
    })
}

pub fn worker_stdin_arguments(
    edge: &std::ffi::OsStr,
    hint: Option<&std::ffi::OsStr>,
) -> Result<()> {
    let edge = edge
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid preview size."))?;
    if !matches!(edge, 256 | 512) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Preview edge must be256 or512 pixels.",
        ));
    }
    let hint = match hint {
        Some(value) => {
            PreviewFormat::from_code(value.to_str().and_then(|s| s.parse().ok()).ok_or_else(
                || AppError::new(ErrorCode::InvalidInput, "Invalid preview format hint."),
            )?)?
        }
        None => PreviewFormat::Auto,
    };
    worker_stdin(edge, hint)
}

fn worker_stdin(edge: u32, hint: PreviewFormat) -> Result<()> {
    use std::io::Read;
    let report = (|| -> Result<MediaReport> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Preview input exceeds 32 MiB.",
            ));
        }
        decode_bytes(&bytes, edge, hint)
    })();
    // Typed errors cross the bounded worker channel; private parser causes do not.
    serde_json::to_writer(std::io::stdout().lock(), &report).map_err(|e| {
        AppError::new(ErrorCode::WorkerFailed, "Could not return preview output.").caused_by(e)
    })
}

fn decode_error(error: image::ImageError) -> AppError {
    AppError::new(
        ErrorCode::WorkerFailed,
        "The image could not be decoded; its original is unchanged.",
    )
    .caused_by(error)
}

pub struct MediaWorker {
    scheduler: JobScheduler,
    timeout: Duration,
}

impl MediaWorker {
    pub fn new(timeout: Duration) -> Result<Self> {
        Ok(Self {
            scheduler: JobScheduler::new(1)?,
            timeout,
        })
    }

    pub async fn thumbnail(
        &self,
        executable: &Path,
        input: &Path,
        working_directory: &Path,
    ) -> Result<MediaReport> {
        let _permit = self.scheduler.try_start()?;
        let mut child = Command::new(executable)
            .arg("--media-worker")
            .arg(input)
            .current_dir(working_directory)
            .env_clear()
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut stdout = child.stdout.take().ok_or_else(|| {
            AppError::new(ErrorCode::WorkerFailed, "Decoder stdout was unavailable.")
        })?;
        let pid = child
            .id()
            .ok_or_else(|| AppError::new(ErrorCode::WorkerFailed, "Missing decoder identity."))?;
        let processing = async {
            let mut bytes = Vec::new();
            (&mut stdout)
                .take(MAX_OUTPUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .await?;
            if bytes.len() as u64 > MAX_OUTPUT_BYTES {
                return Err(AppError::new(
                    ErrorCode::WorkerFailed,
                    "Decoder exceeded its output budget.",
                ));
            }
            let exit = child.wait().await?;
            if !exit.success() {
                return Err(AppError::new(
                    ErrorCode::WorkerFailed,
                    "Decoder exited unsuccessfully; a new worker may retry the next item.",
                ));
            }
            serde_json::from_slice(&bytes).map_err(|error| {
                AppError::new(ErrorCode::WorkerFailed, "Decoder returned invalid data.")
                    .caused_by(error)
            })
        };
        let outcome = tokio::time::timeout(self.timeout, async {
            tokio::select! {
                result=processing=>result,
                error=watch_decoder(pid)=>Err(error),
            }
        })
        .await;
        match outcome {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(error)) => {
                let _ = child.kill().await;
                Err(error)
            }
            Err(_) => {
                let _ = child.kill().await;
                Err(AppError::new(
                    ErrorCode::WorkerTimeout,
                    "Decoder timed out; the original is unchanged.",
                ))
            }
        }
    }
}

/// Sample only the just-spawned decoder child. This is not an OS sandbox or an
/// instantaneous memory quota; the existing time/input/pixel/output limits stay.
pub(crate) async fn watch_decoder(pid: u32) -> AppError {
    #[cfg(target_os = "macos")]
    {
        watch_decoder_limit(pid, 512 * 1024 * 1024).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        std::future::pending().await
    }
}

#[cfg(target_os = "macos")]
async fn watch_decoder_limit(pid: u32, budget: u64) -> AppError {
    loop {
        let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
        let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
        // SAFETY: libproc writes at most the supplied SDK-defined struct size.
        // Read no fields unless its return confirms the complete structure.
        let count = unsafe {
            libc::proc_pidinfo(
                pid as i32,
                libc::PROC_PIDTASKINFO,
                0,
                info.as_mut_ptr().cast(),
                size,
            )
        };
        if count == size {
            // SAFETY: checked full initialization by the kernel above.
            let rss = unsafe { info.assume_init() }.pti_resident_size;
            crate::performance::record(
                crate::performance::Metric::WorkerRss,
                crate::performance::Endpoint::Worker,
                0,
                rss,
            );
            if rss > budget {
                return AppError::new(
                    ErrorCode::WorkerFailed,
                    "The preview decoder exceeded its memory budget; the original is unchanged.",
                );
            }
        } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return AppError::new(
                ErrorCode::WorkerFailed,
                "The preview resource monitor is unavailable; decoding stopped.",
            );
        }
        // ESRCH means this owned process already exited: let its output/wait
        // future report the outcome rather than inventing a decoder failure.
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub fn worker_main(path: &Path) -> Result<()> {
    let report = decode_thumbnail(path)?;
    serde_json::to_writer(std::io::stdout().lock(), &report).map_err(|error| {
        AppError::new(ErrorCode::WorkerFailed, "Could not return decoder output.").caused_by(error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    #[tokio::test]
    #[cfg(target_os = "macos")]
    async fn owned_child_rss_watchdog_detects_growth_and_child_is_reaped() {
        let mut child = Command::new("/usr/bin/perl")
            .args(["-e", "$a=\"x\"x(32*1024*1024);sleep 5"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            watch_decoder_limit(child.id().unwrap(), 16 * 1024 * 1024),
        )
        .await;
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        let error = result.unwrap();
        assert_eq!(error.code, ErrorCode::WorkerFailed);
        assert!(error.message.contains("memory budget"));
    }
    #[test]
    fn worker_hint_protocol_rejects_arbitrary_types_and_unknown_codes_before_reading_input() {
        use std::ffi::OsStr;
        for hint in ["public.tiff", "../../file", "9999"] {
            assert_eq!(
                worker_stdin_arguments(OsStr::new("256"), Some(OsStr::new(hint)))
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidInput
            );
        }
        assert_eq!(
            worker_stdin_arguments(OsStr::new("12000"), None)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn native_orientation_matches_independent_rust_transform_for_all_eight_exif_values() {
        for orientation in 1..=8 {
            let mut bytes = Vec::new();
            let pixels = image::RgbImage::from_fn(24, 12, |x, y| {
                if x < 12 && y < 6 {
                    image::Rgb([220, 20, 10])
                } else if y < 6 {
                    image::Rgb([10, 180, 30])
                } else {
                    image::Rgb([10, 30, 220])
                }
            });
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 100);
            encoder
                .set_exif_metadata(vec![
                    73,
                    73,
                    42,
                    0,
                    8,
                    0,
                    0,
                    0,
                    1,
                    0,
                    18,
                    1,
                    3,
                    0,
                    1,
                    0,
                    0,
                    0,
                    orientation,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ])
                .unwrap();
            encoder
                .write_image(pixels.as_raw(), 24, 12, image::ExtendedColorType::Rgb8)
                .unwrap();
            let expected = decode_reader(
                ImageReader::new(Cursor::new(&bytes))
                    .with_guessed_format()
                    .unwrap(),
                256,
            )
            .unwrap();
            let native = crate::native_preview::decode(&bytes, 256, PreviewFormat::Auto).unwrap();
            assert_eq!(
                (native.thumbnail_width, native.thumbnail_height),
                (expected.thumbnail_width, expected.thumbnail_height),
                "orientation {orientation}"
            );
            let a = image::load_from_memory(&expected.png).unwrap().to_rgb8();
            let b = image::load_from_memory(&native.png).unwrap().to_rgb8();
            for (a, b) in a.pixels().zip(b.pixels()) {
                for (a, b) in a.0.into_iter().zip(b.0) {
                    assert!(
                        a.abs_diff(b) <= 10,
                        "orientation {orientation}: color/coordinate mismatch"
                    );
                }
            }
        }
    }
    #[test]
    fn exif_rotation_is_applied_and_icc_is_explicitly_unsupported() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("oriented.jpg");
        let pixels = image::RgbImage::from_pixel(4, 2, image::Rgb([20, 80, 50]));
        let mut encoded = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut encoded);
        // Little-endian TIFF with one SHORT Orientation=6 entry (90° clockwise).
        encoder
            .set_exif_metadata(vec![
                73, 73, 42, 0, 8, 0, 0, 0, 1, 0, 18, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
            ])
            .unwrap();
        encoder
            .write_image(pixels.as_raw(), 4, 2, image::ExtendedColorType::Rgb8)
            .unwrap();
        std::fs::write(&path, &encoded).unwrap();
        let report = decode_thumbnail(&path).unwrap();
        assert_eq!((report.thumbnail_width, report.thumbnail_height), (2, 4));
        assert!(report.orientation_applied);
        assert_eq!(std::fs::read(&path).unwrap(), encoded);

        let mut encoded = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut encoded);
        encoder.set_icc_profile(vec![0; 128]).unwrap();
        encoder
            .write_image(pixels.as_raw(), 4, 2, image::ExtendedColorType::Rgb8)
            .unwrap();
        std::fs::write(&path, &encoded).unwrap();
        assert_eq!(
            decode_thumbnail(&path).unwrap_err().code,
            ErrorCode::Unsupported
        );
        assert_eq!(std::fs::read(&path).unwrap(), encoded);
    }
    #[test]
    fn generated_image_is_bounded_and_source_bytes_survive() {
        let temp = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "-weird%港📷.png"
        } else {
            "-weird\"%\n港📷.png"
        };
        let path = temp.path().join(name);
        image::RgbImage::from_pixel(640, 320, image::Rgb([30, 130, 90]))
            .save(&path)
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let report = decode_thumbnail(&path).unwrap();
        assert_eq!((report.source_width, report.source_height), (640, 320));
        assert_eq!(
            (report.thumbnail_width, report.thumbnail_height),
            (256, 128)
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&path, b"malformed").unwrap();
        assert_eq!(
            decode_thumbnail(&path).unwrap_err().code,
            ErrorCode::WorkerFailed
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"malformed");
    }
}

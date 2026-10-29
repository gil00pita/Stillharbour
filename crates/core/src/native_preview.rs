//! Platform codec bridge; invoked only by the short-lived decoder child.
use crate::{
    domain::{AppError, ErrorCode, Result},
    media::MediaReport,
};
use std::io::Cursor;

#[repr(C)]
#[derive(Default)]
struct Preview {
    source_width: u32,
    source_height: u32,
    width: u32,
    height: u32,
    orientation: u32,
    bytes_read: u64,
}
unsafe extern "C" {
    fn sh_imageio_file_preview(
        fd: i32,
        edge: u32,
        hint: u32,
        rgba: *mut u8,
        capacity: usize,
        result: *mut Preview,
    ) -> i32;
    fn sh_valid_icc(bytes: *const u8, length: usize) -> i32;
    fn sh_imageio_preview(
        input: *const u8,
        length: usize,
        edge: u32,
        hint: u32,
        rgba: *mut u8,
        capacity: usize,
        result: *mut Preview,
    ) -> i32;
}

pub(crate) fn valid_icc(bytes: &[u8]) -> bool {
    // SAFETY: synchronous read-only validation; the bridge bounds the input and
    // releases its native objects before this borrowed slice can be dropped.
    unsafe { sh_valid_icc(bytes.as_ptr(), bytes.len()) != 0 }
}

pub(crate) fn decode(
    bytes: &[u8],
    edge: u32,
    hint: crate::media::PreviewFormat,
) -> Result<MediaReport> {
    if !crate::rendition::supported_edge(edge) || bytes.len() as u64 > crate::media::MAX_INPUT_BYTES
    {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Preview exceeds its input or size budget.",
        ));
    }
    let mut rgba = vec![0u8; edge as usize * edge as usize * 4];
    let mut report = Preview::default();
    // SAFETY: all buffers remain live and disjoint for this synchronous call.
    // The C bridge validates lengths and dimensions before writing caller-owned
    // storage; it owns/releases every CoreFoundation object and never opens paths.
    let status = unsafe {
        sh_imageio_preview(
            bytes.as_ptr(),
            bytes.len(),
            edge,
            hint as u32,
            rgba.as_mut_ptr(),
            rgba.len(),
            &mut report,
        )
    };
    encode(status, report, rgba, edge)
}

pub(crate) fn decode_file(
    file: &std::fs::File,
    edge: u32,
    hint: crate::media::PreviewFormat,
) -> Result<MediaReport> {
    use std::{io::Read, os::fd::AsRawFd};
    if !crate::rendition::supported_edge(edge)
        || !file.metadata()?.is_file()
        || file.metadata()?.len() > crate::media::MAX_NATIVE_FILE_BYTES
    {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "This file exceeds the bounded native preview input limit.",
        ));
    }
    let before = crate::library::native::revision(file)?;
    // Header-only validation retains the strict explicit JPEG/PNG ICC policy.
    // Never allocate an image-sized Rust pixel buffer on this path.
    let mut prefix = Vec::new();
    file.try_clone()?
        .take(crate::media::MAX_INPUT_BYTES)
        .read_to_end(&mut prefix)?;
    let header_bytes = prefix.len() as u64;
    if let Ok(format @ (image::ImageFormat::Jpeg | image::ImageFormat::Png)) =
        image::guess_format(&prefix)
    {
        use image::ImageDecoder;
        let mut reader = image::ImageReader::with_format(Cursor::new(prefix), format);
        let mut limits = image::Limits::default();
        let side = if format == image::ImageFormat::Jpeg {
            crate::rendition::NATIVE_JPEG_MAX_SIDE
        } else {
            12000
        };
        limits.max_image_width = Some(side);
        limits.max_image_height = Some(side);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().map_err(crate::media::decode_error)?;
        let (width, height) = decoder.dimensions();
        if u64::from(width) * u64::from(height) > crate::rendition::NATIVE_MAX_SOURCE_PIXELS {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "This image exceeds the preview dimension or memory budget.",
            ));
        }
        if let Some(profile) = decoder.icc_profile().map_err(crate::media::decode_error)?
            && !valid_icc(&profile)
        {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "The embedded color profile is invalid or exceeds the preview profile budget.",
            ));
        }
    }
    let mut rgba = vec![0u8; edge as usize * edge as usize * 4];
    let mut report = Preview::default();
    // SAFETY: a borrowed read-only regular fd and disjoint live output buffers.
    // C duplicates the fd and owns its bounded provider through releaseInfo.
    let status = unsafe {
        sh_imageio_file_preview(
            file.as_raw_fd(),
            edge,
            hint as u32,
            rgba.as_mut_ptr(),
            rgba.len(),
            &mut report,
        )
    };
    if crate::library::native::revision(file)? != before {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "The original changed during preview generation.",
        ));
    }
    let mut result = encode(status, report, rgba, edge)?;
    result.input_read_bytes += header_bytes;
    Ok(result)
}

fn encode(status: i32, report: Preview, mut rgba: Vec<u8>, edge: u32) -> Result<MediaReport> {
    if status != 0 {
        let (code, message) = match status {
            1 => (
                ErrorCode::Unsupported,
                "This format has no enabled native preview adapter on this Mac.",
            ),
            3 => (
                ErrorCode::Unsupported,
                "This image exceeds the preview dimension or memory budget.",
            ),
            4 => (
                ErrorCode::Unsupported,
                "A color-managed sRGB preview could not be created.",
            ),
            5 => (
                ErrorCode::WorkerFailed,
                "This camera variant has no decodable native preview on this Mac.",
            ),
            6 => (
                ErrorCode::WorkerFailed,
                "The native codec could not read this image's dimensions.",
            ),
            _ => (
                ErrorCode::WorkerFailed,
                "This Mac could not decode a preview of this file; its original is unchanged.",
            ),
        };
        return Err(AppError::new(code, message));
    }
    if report.width == 0 || report.height == 0 || report.width > edge || report.height > edge {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "The native decoder returned invalid dimensions.",
        ));
    }
    rgba.truncate(report.width as usize * report.height as usize * 4);
    let image = image::RgbaImage::from_raw(report.width, report.height, rgba)
        .ok_or_else(|| AppError::new(ErrorCode::WorkerFailed, "Invalid preview pixels."))?;
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Could not encode preview pixels."))?;
    if output.get_ref().len() > crate::rendition::png_budget(edge)? {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "The preview exceeded its output budget.",
        ));
    }
    Ok(MediaReport {
        source_width: report.source_width,
        source_height: report.source_height,
        thumbnail_width: report.width,
        thumbnail_height: report.height,
        orientation_applied: report.orientation != 1,
        color_policy: "macOS ImageIO / ColorSync → sRGB RGBA8 SDR; primary image / first animation frame, preview only."
            .into(),
        png: output.into_inner(),
        display_dimensions: Some(if report.orientation >= 5 { (report.source_height, report.source_width) } else { (report.source_width, report.source_height) }),
        input_read_bytes: report.bytes_read,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(image: image::RgbaImage) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }
    #[test]
    fn rgba_order_top_bottom_alpha_and_no_upscale_are_preserved() {
        let mut image = image::RgbaImage::from_pixel(4, 2, image::Rgba([220, 20, 10, 255]));
        image.put_pixel(0, 1, image::Rgba([10, 30, 200, 128]));
        image.put_pixel(1, 1, image::Rgba([0, 0, 0, 0]));
        let report = decode(&png(image.clone()), 256, crate::media::PreviewFormat::Auto).unwrap();
        assert_eq!((report.thumbnail_width, report.thumbnail_height), (4, 2));
        let decoded = image::load_from_memory(&report.png).unwrap().to_rgba8();
        for (a, b) in decoded.pixels().zip(image.pixels()) {
            for (v, w) in a.0.into_iter().zip(b.0) {
                assert!(
                    v.abs_diff(w) <= 2,
                    "RGBA or coordinate mismatch: {a:?} vs {b:?}"
                );
            }
        }
    }
    #[test]
    fn file_provider_rejects_writable_handles_and_keeps_tiny_dimensions_and_rgba() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("owned.png");
        let pixels = png(image::RgbaImage::from_pixel(
            64,
            32,
            image::Rgba([20, 60, 200, 128]),
        ));
        std::fs::write(&path, &pixels).unwrap();
        let file = crate::filesystem::open_readonly(&path).unwrap();
        let report = decode_file(&file, 32, crate::media::PreviewFormat::Auto).unwrap();
        assert_eq!((report.thumbnail_width, report.thumbnail_height), (32, 16));
        assert_eq!(report.display_dimensions, Some((64, 32)));
        let pixel = image::load_from_memory(&report.png)
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)
            .0;
        for (actual, expected) in pixel.into_iter().zip([20u8, 60, 200, 128]) {
            assert!(actual.abs_diff(expected) <= 2);
        }
        let writable = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            decode_file(&writable, 32, crate::media::PreviewFormat::Auto)
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        assert_eq!(std::fs::read(path).unwrap(), pixels);
        use image::ImageEncoder;
        let mut bad = Vec::new();
        let mut encoder = image::codecs::png::PngEncoder::new(&mut bad);
        encoder.set_icc_profile(vec![0; 128]).unwrap();
        encoder
            .write_image(&[20, 60, 200], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let bad_path = temp.path().join("invalid-profile.png");
        std::fs::write(&bad_path, &bad).unwrap();
        let bad_file = crate::filesystem::open_readonly(&bad_path).unwrap();
        assert_eq!(
            decode_file(&bad_file, 32, crate::media::PreviewFormat::Auto)
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        assert_eq!(std::fs::read(bad_path).unwrap(), bad);
    }
    #[test]
    fn file_backed_tall_jpeg_downsamples_without_a_full_raster() {
        use image::ImageEncoder;
        let owned = tempfile::tempdir().unwrap();
        let pixels = image::RgbImage::from_pixel(64, 12288, image::Rgb([20, 60, 130]));
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut bytes)
            .write_image(pixels.as_raw(), 64, 12288, image::ExtendedColorType::Rgb8)
            .unwrap();
        let path = owned.path().join("generated-tall.jpg");
        std::fs::write(&path, &bytes).unwrap();
        let file = crate::filesystem::open_readonly(&path).unwrap();
        let result = decode_file(&file, 32, crate::media::PreviewFormat::Auto);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let report = result.unwrap();
        assert_eq!(report.display_dimensions, Some((64, 12288)));
        assert_eq!((report.thumbnail_width, report.thumbnail_height), (1, 32));
        crate::rendition::validate(&report, 32).unwrap();
    }

    #[test]
    fn file_jpeg_still_rejects_excessive_sides_pixels_and_other_format_limits() {
        use image::ImageEncoder;
        let owned = tempfile::tempdir().unwrap();
        let mut seed = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut seed)
            .write_image(&[20, 60, 130], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let sof = seed
            .windows(2)
            .position(|pair| pair == [0xff, 0xc0])
            .unwrap();
        for (width, height) in [(16385u16, 1u16), (12000, 12001)] {
            let mut bytes = seed.clone();
            bytes[sof + 5..sof + 7].copy_from_slice(&height.to_be_bytes());
            bytes[sof + 7..sof + 9].copy_from_slice(&width.to_be_bytes());
            let path = owned.path().join(format!("generated-{width}-{height}.jpg"));
            std::fs::write(&path, &bytes).unwrap();
            let file = crate::filesystem::open_readonly(&path).unwrap();
            let error = decode_file(&file, 32, crate::media::PreviewFormat::Auto).unwrap_err();
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert_eq!(
                error.message,
                "This image exceeds the preview dimension or memory budget."
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let bytes = png(image::RgbaImage::from_pixel(
            1,
            12001,
            image::Rgba([20, 60, 130, 255]),
        ));
        let path = owned.path().join("generated-tall.png");
        std::fs::write(&path, &bytes).unwrap();
        let file = crate::filesystem::open_readonly(&path).unwrap();
        assert_eq!(
            decode_file(&file, 32, crate::media::PreviewFormat::Auto)
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn malformed_and_over_dimension_inputs_fail_closed() {
        assert!(decode(b"invalid", 256, crate::media::PreviewFormat::Auto).is_err());
        let wide = png(image::RgbaImage::from_pixel(
            12001,
            1,
            image::Rgba([1, 2, 3, 255]),
        ));
        assert_eq!(
            decode(&wide, 256, crate::media::PreviewFormat::Auto)
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        assert!(!valid_icc(&[0; 128]));
    }
}

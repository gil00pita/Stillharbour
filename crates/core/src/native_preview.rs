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
}
unsafe extern "C" {
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
    if !matches!(edge, 256 | 512) || bytes.len() as u64 > crate::media::MAX_INPUT_BYTES {
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
    if output.get_ref().len() as u64 > crate::media::MAX_OUTPUT_BYTES {
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

//! Bounded derived-image tiers and the private decoder response protocol.
use crate::{
    domain::{AppError, ErrorCode, Result},
    media::MediaReport,
};
use serde::{Deserialize, Serialize};
use std::io::Write;

pub const EDGES: &[u32] = &[32, 256, 512, 1024, 2048];
pub const GALLERY_MAX_EDGE: u32 = 1024;
pub const VIEWER_EDGE: u32 = 2048;
pub const GALLERY_DECODED_BYTES: usize = 128 * 1024 * 1024;
// Source geometry envelope for file-backed JPEG downsampling; pixel count
// stays below the previous 12000×12000 envelope. Other adapters keep 12000.
pub const NATIVE_JPEG_MAX_SIDE: u32 = 16384;
pub const NATIVE_MAX_SOURCE_PIXELS: u64 = 12000 * 12000;
const MAGIC: &[u8; 8] = b"SHRP\0\0\0\x02";
const HEADER_LIMIT: usize = 4096;

pub fn supported_edge(edge: u32) -> bool {
    EDGES.contains(&edge)
}

pub fn png_budget(edge: u32) -> Result<usize> {
    match edge {
        32 | 256 | 512 => Ok(2 * 1024 * 1024),
        1024 => Ok(5 * 1024 * 1024),
        2048 => Ok(17 * 1024 * 1024),
        _ => Err(AppError::new(
            ErrorCode::InvalidInput,
            "Unknown rendition tier.",
        )),
    }
}
pub fn wire_budget(edge: u32) -> Result<u64> {
    Ok((png_budget(edge)? + HEADER_LIMIT + 12) as u64)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    source_width: u32,
    source_height: u32,
    thumbnail_width: u32,
    thumbnail_height: u32,
    orientation_applied: bool,
    color_policy: String,
    display_dimensions: Option<(u32, u32)>,
    input_read_bytes: u64,
    png_bytes: usize,
}
impl From<&MediaReport> for Header {
    fn from(r: &MediaReport) -> Self {
        Self {
            source_width: r.source_width,
            source_height: r.source_height,
            thumbnail_width: r.thumbnail_width,
            thumbnail_height: r.thumbnail_height,
            orientation_applied: r.orientation_applied,
            color_policy: r.color_policy.clone(),
            display_dimensions: r.display_dimensions,
            input_read_bytes: r.input_read_bytes,
            png_bytes: r.png.len(),
        }
    }
}
fn invalid() -> AppError {
    AppError::new(
        ErrorCode::WorkerFailed,
        "The preview worker returned an invalid rendition frame.",
    )
}

pub fn validate(report: &MediaReport, edge: u32) -> Result<()> {
    let png = &report.png;
    if png.len() > png_budget(edge)?
        || png.len() < 33
        || !png.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR")
        || report.thumbnail_width == 0
        || report.thumbnail_height == 0
        || report.thumbnail_width > edge
        || report.thumbnail_height > edge
        || report.source_width == 0
        || report.source_height == 0
        || report.source_width > NATIVE_JPEG_MAX_SIDE
        || report.source_height > NATIVE_JPEG_MAX_SIDE
        || u64::from(report.source_width) * u64::from(report.source_height)
            > NATIVE_MAX_SOURCE_PIXELS
        || u32::from_be_bytes(png[16..20].try_into().map_err(|_| invalid())?)
            != report.thumbnail_width
        || u32::from_be_bytes(png[20..24].try_into().map_err(|_| invalid())?)
            != report.thumbnail_height
        || png[24] != 8
        || png[25] != 6
        || report.input_read_bytes > crate::media::MAX_NATIVE_FILE_BYTES
        || report.color_policy.len() > 1024
    {
        return Err(invalid());
    }
    let longest = report.source_width.max(report.source_height);
    if report.thumbnail_width > longest || report.thumbnail_height > longest {
        return Err(invalid());
    }
    if let Some((width, height)) = report.display_dimensions
        && (![
            (report.source_width, report.source_height),
            (report.source_height, report.source_width),
        ]
        .contains(&(width, height))
            || report.thumbnail_width > width
            || report.thumbnail_height > height)
    {
        return Err(invalid());
    }
    Ok(())
}

pub fn write_frame(mut output: impl Write, report: Result<MediaReport>, edge: u32) -> Result<()> {
    png_budget(edge)?;
    let report = report.and_then(|r| {
        validate(&r, edge)?;
        Ok(r)
    });
    let header: Result<Header> = report.as_ref().map(Header::from).map_err(Clone::clone);
    let encoded = serde_json::to_vec(&header).map_err(|_| invalid())?;
    if encoded.len() > HEADER_LIMIT {
        return Err(invalid());
    }
    output.write_all(MAGIC)?;
    output.write_all(&(encoded.len() as u32).to_le_bytes())?;
    output.write_all(&encoded)?;
    if let Ok(report) = report {
        output.write_all(&report.png)?;
    }
    Ok(())
}

pub fn read_frame(bytes: &[u8], edge: u32) -> Result<MediaReport> {
    if bytes.len() < 12 || bytes.len() as u64 > wire_budget(edge)? || &bytes[..8] != MAGIC {
        return Err(invalid());
    }
    let length = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid())?) as usize;
    if length > HEADER_LIMIT || length > bytes.len() - 12 {
        return Err(invalid());
    }
    let header: Result<Header> =
        serde_json::from_slice(&bytes[12..12 + length]).map_err(|_| invalid())?;
    let png = &bytes[12 + length..];
    let header = match header {
        Ok(header) => header,
        Err(error) if png.is_empty() && error.message.len() <= 2048 => return Err(error),
        Err(_) => return Err(invalid()),
    };
    if header.png_bytes != png.len() {
        return Err(invalid());
    }
    let report = MediaReport {
        source_width: header.source_width,
        source_height: header.source_height,
        thumbnail_width: header.thumbnail_width,
        thumbnail_height: header.thumbnail_height,
        orientation_applied: header.orientation_applied,
        color_policy: header.color_policy,
        display_dimensions: header.display_dimensions,
        input_read_bytes: header.input_read_bytes,
        png: png.to_vec(),
    };
    validate(&report, edge)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> MediaReport {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 1, image::Rgba([20, 40, 60, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        MediaReport {
            source_width: 2,
            source_height: 1,
            thumbnail_width: 2,
            thumbnail_height: 1,
            orientation_applied: false,
            color_policy: "fixture sRGB".into(),
            png: png.into_inner(),
            display_dimensions: Some((2, 1)),
            input_read_bytes: 100,
        }
    }
    #[test]
    fn framed_png_preserves_exact_bytes_and_rejects_truncation_version_lengths_and_dimensions() {
        let expected = report().png;
        let mut bytes = Vec::new();
        write_frame(&mut bytes, Ok(report()), 2048).unwrap();
        assert_eq!(read_frame(&bytes, 2048).unwrap().png, expected);
        for length in 0..bytes.len() {
            assert!(read_frame(&bytes[..length], 2048).is_err());
        }
        let mut wrong = bytes.clone();
        wrong[7] = 3;
        assert!(read_frame(&wrong, 2048).is_err());
        let mut wrong = bytes.clone();
        wrong[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_frame(&wrong, 2048).is_err());
        let mut wrong = bytes.clone();
        wrong.push(0);
        assert!(read_frame(&wrong, 2048).is_err());
        let mut wrong = report();
        wrong.thumbnail_width = 3;
        assert!(validate(&wrong, 2048).is_err());
        let mut wrong = report();
        wrong.display_dimensions = Some((12001, 1));
        assert!(validate(&wrong, 2048).is_err());
        let mut wrong = report();
        wrong.source_width = 1;
        wrong.source_height = 1;
        wrong.display_dimensions = None;
        assert!(validate(&wrong, 2048).is_err());
        assert!(read_frame(&bytes, 2049).is_err());
    }
    #[test]
    fn frame_geometry_keeps_both_axis_and_total_source_pixel_caps() {
        let mut value = report();
        value.source_width = 8192;
        value.source_height = 12288;
        value.display_dimensions = None;
        assert!(validate(&value, 32).is_ok());
        value.source_width = 16385;
        value.source_height = 1;
        assert!(validate(&value, 32).is_err());
        value.source_width = 12000;
        value.source_height = 12001;
        assert!(validate(&value, 32).is_err());
    }

    #[test]
    fn typed_errors_have_no_image_body_and_png_limits_are_tier_specific() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            Err(AppError::new(ErrorCode::Unsupported, "Fixture unavailable")),
            1024,
        )
        .unwrap();
        assert_eq!(
            read_frame(&bytes, 1024).unwrap_err().code,
            ErrorCode::Unsupported
        );
        bytes.push(0);
        assert_eq!(
            read_frame(&bytes, 1024).unwrap_err().code,
            ErrorCode::WorkerFailed
        );
        assert_eq!(png_budget(512).unwrap(), 2 * 1024 * 1024);
        assert_eq!(png_budget(2048).unwrap(), 17 * 1024 * 1024);
        assert!(png_budget(4096).is_err());
    }
}

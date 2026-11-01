//! Bounded helper-process raster inspection. No filesystem path input or writes.
use super::model::ContentState;
use crate::domain::{AppError, ErrorCode, Result};
use image::{ImageDecoder, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};

pub const INPUT_LIMIT: u64 = 32 * 1024 * 1024;
pub const QUICK_LIMIT: u64 = 64 * 1024;
pub const OUTPUT_LIMIT: u64 = 16 * 1024;
pub const DEEP_POLICY: &str = "inspection-static-raster-image-0.25.10-v1";
pub const PIXEL_POLICY: &str = "inspection-png8-straight-color-provenance-image-0.25.10-v2";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecodeMode {
    Quick,
    Deep,
    Pixels,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatHint {
    Image,
    Raw,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecodeReport {
    pub state: ContentState,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub policy: String,
    pub coverage: String,
    pub pixel_signature: Option<String>,
    pub pixel_digest: Option<String>,
    pub error: Option<AppError>,
}
fn outcome(state: ContentState, policy: &str, coverage: &str) -> DecodeReport {
    DecodeReport {
        state,
        width: None,
        height: None,
        format: None,
        policy: policy.into(),
        coverage: coverage.into(),
        pixel_signature: None,
        pixel_digest: None,
        error: None,
    }
}
fn unsupported(message: &str, mode: DecodeMode) -> DecodeReport {
    let mut report = outcome(
        ContentState::Unsupported,
        if mode == DecodeMode::Pixels {
            PIXEL_POLICY
        } else {
            DEEP_POLICY
        },
        message,
    );
    report.error = Some(AppError::new(ErrorCode::Unsupported, message));
    report
}
fn decode_failure(error: image::ImageError, mode: DecodeMode, complete: bool) -> DecodeReport {
    if matches!(
        error,
        image::ImageError::Limits(_) | image::ImageError::Unsupported(_)
    ) || !complete
    {
        return unsupported(
            "Format, precision or header exceeds this inspection's supported coverage/budget.",
            mode,
        );
    }
    let mut report = outcome(
        if mode == DecodeMode::Quick {
            ContentState::SuspectedCorrupt
        } else {
            ContentState::ConfirmedDecodeFailure
        },
        DEEP_POLICY,
        "The selected image parser could not complete its declared coverage; no repair or deletion was attempted.",
    );
    report.error = Some(
        AppError::new(
            ErrorCode::WorkerFailed,
            "Image decoding failed within the supported path.",
        )
        .caused_by(error),
    );
    report
}
fn png_policy(bytes: &[u8]) -> std::result::Result<(Vec<u8>, String), &'static str> {
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("PNG header is incomplete.");
    }
    if bytes[24] != 8 || ![2, 6].contains(&bytes[25]) {
        return Err(
            "Normalized pixels support only RGB8/RGBA8 PNG; palette, grayscale and higher precision remain unsupported.",
        );
    }
    let mut decoded_input = bytes[..8].to_vec();
    let mut cursor = 8usize;
    let mut srgb = None;
    let mut ended = false;
    let mut time_seen = false;
    let mut idat_seen = false;
    let mut idat_ended = false;
    while cursor < bytes.len() {
        let header = bytes
            .get(cursor..cursor + 8)
            .ok_or("PNG chunk header is incomplete.")?;
        let length = u32::from_be_bytes(
            header[..4]
                .try_into()
                .map_err(|_| "PNG chunk length is invalid.")?,
        ) as usize;
        let end = cursor
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
            .ok_or("PNG chunk exceeds its budget.")?;
        let chunk = bytes.get(cursor..end).ok_or("PNG chunk is incomplete.")?;
        let kind = &header[4..8];
        let stored_crc = u32::from_be_bytes(
            chunk[chunk.len() - 4..]
                .try_into()
                .map_err(|_| "PNG CRC is incomplete.")?,
        );
        if crc32fast::hash(&chunk[4..chunk.len() - 4]) != stored_crc {
            return Err("PNG chunk CRC is invalid; normalization is unsupported for this input.");
        }
        if cursor == 8 && (kind != b"IHDR" || length != 13) {
            return Err("PNG must begin with one complete IHDR.");
        }
        if idat_seen && kind != b"IDAT" {
            idat_ended = true;
        }
        match kind {
            b"IHDR" => {
                if cursor != 8 || length != 13 {
                    return Err("PNG image header is duplicated or malformed.");
                }
                decoded_input.extend_from_slice(chunk);
            }
            b"IDAT" => {
                if idat_ended {
                    return Err("PNG image-data chunks are not contiguous.");
                }
                idat_seen = true;
                decoded_input.extend_from_slice(chunk);
            }
            b"IEND" => {
                if length != 0 || !idat_seen {
                    return Err("PNG image-data/trailer structure is incomplete.");
                }
                decoded_input.extend_from_slice(chunk);
            }
            b"sRGB" => {
                if length != 1 || chunk[8] > 3 || srgb.is_some() || idat_seen {
                    return Err("PNG sRGB intent is invalid.");
                }
                srgb = Some(chunk[8]);
                decoded_input.extend_from_slice(chunk);
            }
            b"tEXt" => {
                let data = &chunk[8..8 + length];
                let separator = data
                    .iter()
                    .position(|byte| *byte == 0)
                    .ok_or("PNG text keyword separator is missing.")?;
                if length > 4096
                    || data[separator + 1..].contains(&0)
                    || !matches!(
                        &data[..separator],
                        b"Title"
                            | b"Author"
                            | b"Description"
                            | b"Copyright"
                            | b"Creation Time"
                            | b"Software"
                            | b"Disclaimer"
                            | b"Warning"
                            | b"Source"
                            | b"Comment"
                            | b"Collection"
                    )
                {
                    return Err(
                        "Only bounded uncompressed text with declared descriptive PNG keywords can be excluded; XMP and unknown text semantics are unsupported.",
                    );
                }
            }
            b"tIME" => {
                if length != 7 || time_seen {
                    return Err("PNG time component is malformed or duplicated.");
                }
                let data = &chunk[8..15];
                let year = u16::from_be_bytes([data[0], data[1]]);
                if year == 0
                    || chrono::NaiveDate::from_ymd_opt(
                        i32::from(year),
                        u32::from(data[2]),
                        u32::from(data[3]),
                    )
                    .is_none()
                    || data[4] > 23
                    || data[5] > 59
                    || data[6] > 60
                {
                    return Err("PNG time component has invalid fields.");
                }
                time_seen = true;
            }
            _ => {
                return Err(
                    "PNG color, orientation, transparency, animation or unknown components are outside the normalized-pixel policy.",
                );
            }
        }
        cursor = end;
        if kind == b"IEND" {
            ended = true;
            break;
        }
    }
    if !ended || cursor != bytes.len() {
        return Err(
            "PNG trailing or incomplete components are outside the normalized-pixel policy.",
        );
    }
    Ok((
        decoded_input,
        format!(
            "{PIXEL_POLICY}:layout={}:precision=8:alpha={}:srgb-intent={}",
            if bytes[25] == 2 { "rgb" } else { "rgba" },
            if bytes[25] == 2 { "none" } else { "straight" },
            srgb.map_or_else(|| "untagged".to_owned(), |intent| intent.to_string())
        ),
    ))
}
fn animated(bytes: &[u8], format: image::ImageFormat) -> bool {
    if format == image::ImageFormat::Png {
        let mut cursor = 8usize;
        while let Some(header) = bytes.get(cursor..cursor.saturating_add(8)) {
            let length = u32::from_be_bytes(header[..4].try_into().unwrap_or([0; 4])) as usize;
            if matches!(&header[4..8], b"acTL" | b"fcTL" | b"fdAT") {
                return true;
            }
            let Some(next) = cursor
                .checked_add(length)
                .and_then(|value| value.checked_add(12))
            else {
                return true;
            };
            cursor = next;
        }
    } else if format == image::ImageFormat::WebP {
        let mut cursor = 12usize;
        while let Some(header) = bytes.get(cursor..cursor.saturating_add(8)) {
            if matches!(&header[..4], b"ANIM" | b"ANMF") {
                return true;
            }
            let length = u32::from_le_bytes(header[4..8].try_into().unwrap_or([0; 4])) as usize;
            let Some(next) = cursor
                .checked_add(8)
                .and_then(|value| value.checked_add(length))
                .and_then(|value| value.checked_add(length % 2))
            else {
                return true;
            };
            cursor = next;
        }
    }
    false
}
pub fn inspect(bytes: &[u8], mode: DecodeMode, hint: FormatHint, complete: bool) -> DecodeReport {
    if bytes.len() as u64 > INPUT_LIMIT {
        return unsupported("Inspection input exceeds 32 MiB.", mode);
    }
    if hint == FormatHint::Raw {
        return unsupported(
            "RAW image-data validation is unsupported. An embedded preview never establishes full RAW health.",
            mode,
        );
    }
    let format = match image::guess_format(bytes) {
        Ok(format)
            if matches!(
                format,
                image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP
            ) =>
        {
            format
        }
        _ => {
            let mut report =
                unsupported("This file format has no declared inspection decoder.", mode);
            if hint == FormatHint::Image && complete {
                report.state = ContentState::SuspectedCorrupt;
                report.coverage="Expected image header was not recognized; inspect manually, no automatic response.".into();
            }
            return report;
        }
    };
    if mode != DecodeMode::Quick && animated(bytes, format) {
        return unsupported(
            "Animated/multipage coverage is unsupported; a matching first frame is not full-asset validation.",
            mode,
        );
    }
    let normalized = if mode == DecodeMode::Pixels {
        if format != image::ImageFormat::Png {
            return unsupported(
                "Normalized pixels currently support only the declared static PNG8 policy.",
                mode,
            );
        }
        match png_policy(bytes) {
            Ok(value) => Some(value),
            Err(message) => return unsupported(message, mode),
        }
    } else {
        None
    };
    let input = normalized
        .as_ref()
        .map_or(bytes, |(bytes, _)| bytes.as_slice());
    let mut reader = ImageReader::with_format(Cursor::new(input), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoder = match reader.into_decoder() {
        Ok(decoder) => decoder,
        Err(error) => return decode_failure(error, mode, complete),
    };
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 16_000_000 {
        return unsupported(
            "Image exceeds the 16-megapixel bounded validation policy.",
            mode,
        );
    }
    let mut report = outcome(
        if mode == DecodeMode::Quick {
            ContentState::QuickPass
        } else {
            ContentState::DeepPass
        },
        if mode == DecodeMode::Pixels {
            PIXEL_POLICY
        } else {
            DEEP_POLICY
        },
        if mode == DecodeMode::Quick {
            "Signature and decoded header dimensions only; image data/frames were not fully decoded."
        } else {
            "Full static raster decoded within limits; this is not original-state or backup evidence."
        },
    );
    report.width = Some(width);
    report.height = Some(height);
    report.format = Some(format!("{format:?}").to_ascii_lowercase());
    if mode != DecodeMode::Quick {
        let image = match image::DynamicImage::from_decoder(decoder) {
            Ok(image) => image,
            Err(error) => return decode_failure(error, mode, complete),
        };
        if let Some((_, signature)) = normalized {
            let mut hash = blake3::Hasher::new();
            hash.update(signature.as_bytes());
            hash.update(&width.to_le_bytes());
            hash.update(&height.to_le_bytes());
            hash.update(image.as_bytes());
            report.pixel_signature = Some(signature);
            report.pixel_digest = Some(hash.finalize().to_hex().to_string());
            report.coverage="Full static RGB8/RGBA8 PNG encoded samples with straight alpha and preserved sRGB-tag/intent or untagged provenance; no appearance-equivalence proof; whitelisted uncompressed descriptive text/valid time excluded; chunk CRCs checked. XMP/compressed/international text are unsupported. Manual review only.".into();
        }
    }
    report
}
pub fn worker(
    mode: &std::ffi::OsStr,
    hint: &std::ffi::OsStr,
    complete: &std::ffi::OsStr,
) -> Result<()> {
    let mode = match mode.to_str() {
        Some("quick") => DecodeMode::Quick,
        Some("deep") => DecodeMode::Deep,
        Some("pixels") => DecodeMode::Pixels,
        _ => {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Unknown inspection worker mode.",
            ));
        }
    };
    let hint = match hint.to_str() {
        Some("image") => FormatHint::Image,
        Some("raw") => FormatHint::Raw,
        Some("unknown") => FormatHint::Unknown,
        _ => {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Unknown inspection format hint.",
            ));
        }
    };
    let complete = match complete.to_str() {
        Some("complete") => true,
        Some("prefix") => false,
        _ => {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid inspection coverage.",
            ));
        }
    };
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    let report = inspect(&bytes, mode, hint, complete);
    let output = serde_json::to_vec(&report).map_err(|error| {
        AppError::new(
            ErrorCode::WorkerFailed,
            "Could not encode inspection result.",
        )
        .caused_by(error)
    })?;
    if output.len() as u64 > OUTPUT_LIMIT {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "Inspection result exceeds its output limit.",
        ));
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    fn png(color: image::ExtendedColorType) -> Vec<u8> {
        let mut bytes = Vec::new();
        let pixels = vec![42; 2 * 3 * (color.bits_per_pixel() / 8) as usize];
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixels, 2, 3, color)
            .unwrap();
        bytes
    }
    #[test]
    fn headers_full_decode_and_raw_coverage_are_distinct() {
        let bytes = png(image::ExtendedColorType::Rgb8);
        let quick = inspect(&bytes, DecodeMode::Quick, FormatHint::Image, true);
        assert_eq!(quick.state, ContentState::QuickPass);
        assert_eq!((quick.width, quick.height), (Some(2), Some(3)));
        assert!(quick.pixel_digest.is_none());
        assert_eq!(
            inspect(&bytes, DecodeMode::Deep, FormatHint::Image, true).state,
            ContentState::DeepPass
        );
        assert_eq!(
            inspect(&bytes, DecodeMode::Deep, FormatHint::Raw, true).state,
            ContentState::Unsupported
        );
        let idat = bytes.windows(4).position(|part| part == b"IDAT").unwrap();
        let truncated = &bytes[..idat + 4];
        assert_ne!(
            inspect(truncated, DecodeMode::Deep, FormatHint::Image, true).state,
            ContentState::DeepPass
        );
        assert_ne!(
            inspect(truncated, DecodeMode::Deep, FormatHint::Image, false).state,
            ContentState::ConfirmedDecodeFailure
        );
    }
    #[test]
    fn pixel_policy_rejects_precision_and_alpha_layout_confusion() {
        let rgb = inspect(
            &png(image::ExtendedColorType::Rgb8),
            DecodeMode::Pixels,
            FormatHint::Image,
            true,
        );
        let rgba = inspect(
            &png(image::ExtendedColorType::Rgba8),
            DecodeMode::Pixels,
            FormatHint::Image,
            true,
        );
        assert!(rgb.pixel_digest.is_some());
        assert_ne!(rgb.pixel_signature, rgba.pixel_signature);
        assert_eq!(
            inspect(
                &png(image::ExtendedColorType::Rgba16),
                DecodeMode::Pixels,
                FormatHint::Image,
                true
            )
            .state,
            ContentState::Unsupported
        );
    }
    fn insert_chunk(original: &[u8], kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut bytes = original[..33].to_vec();
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(data);
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(data);
        bytes.extend_from_slice(&crc.finalize().to_be_bytes());
        bytes.extend_from_slice(&original[33..]);
        bytes
    }
    #[test]
    fn descriptive_metadata_matches_only_with_valid_crc_and_declared_keywords() {
        let original = png(image::ExtendedColorType::Rgb8);
        let plain = inspect(&original, DecodeMode::Pixels, FormatHint::Image, true);
        assert!(plain.pixel_digest.is_some());
        let caption = insert_chunk(&original, b"tEXt", b"Description\0Generated caption");
        assert_ne!(caption, original);
        assert_eq!(
            plain.pixel_digest,
            inspect(&caption, DecodeMode::Pixels, FormatHint::Image, true).pixel_digest
        );
        let mut broken = caption.clone();
        broken[45] ^= 1;
        assert_eq!(
            inspect(&broken, DecodeMode::Pixels, FormatHint::Image, true).state,
            ContentState::Unsupported
        );
        for (kind, data) in [
            (
                b"tEXt",
                b"XML:com.adobe.xmp\0<tiff:Orientation>6</tiff:Orientation>".as_slice(),
            ),
            (b"tEXt", b"Raw profile type exif\0Orientation=6".as_slice()),
            (
                b"iTXt",
                b"XML:com.adobe.xmp\0\0\0\0\0orientation".as_slice(),
            ),
            (b"zTXt", b"Description\0\0compressed".as_slice()),
            (b"tEXt", b"".as_slice()),
        ] {
            let report = inspect(
                &insert_chunk(&original, kind, data),
                DecodeMode::Pixels,
                FormatHint::Image,
                true,
            );
            assert_eq!(report.state, ContentState::Unsupported);
            assert!(report.pixel_digest.is_none());
        }
    }
    #[test]
    fn color_provenance_and_chunk_structure_are_not_silently_normalized() {
        let original = png(image::ExtendedColorType::Rgb8);
        let plain = inspect(&original, DecodeMode::Pixels, FormatHint::Image, true);
        let tagged = insert_chunk(&original, b"sRGB", &[0]);
        let explicit = inspect(&tagged, DecodeMode::Pixels, FormatHint::Image, true);
        assert!(explicit.pixel_digest.is_some());
        assert_ne!(plain.pixel_signature, explicit.pixel_signature);
        assert_eq!(
            inspect(
                &insert_chunk(&tagged, b"sRGB", &[1]),
                DecodeMode::Pixels,
                FormatHint::Image,
                true
            )
            .state,
            ContentState::Unsupported
        );
        let offset = original
            .windows(4)
            .position(|part| part == b"IDAT")
            .unwrap()
            - 4;
        let length = u32::from_be_bytes(original[offset..offset + 4].try_into().unwrap()) as usize;
        let data = &original[offset + 8..offset + 8 + length];
        let middle = data.len() / 2;
        let mut split = original[..offset].to_vec();
        let append = |target: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]| {
            target.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            target.extend_from_slice(kind);
            target.extend_from_slice(payload);
            let mut crc = crc32fast::Hasher::new();
            crc.update(kind);
            crc.update(payload);
            target.extend_from_slice(&crc.finalize().to_be_bytes());
        };
        append(&mut split, b"IDAT", &data[..middle]);
        append(&mut split, b"tEXt", b"Description\0split IDAT is invalid");
        append(&mut split, b"IDAT", &data[middle..]);
        split.extend_from_slice(&original[offset + length + 12..]);
        assert_eq!(
            inspect(&split, DecodeMode::Pixels, FormatHint::Image, true).state,
            ContentState::Unsupported
        );
    }
    #[test]
    fn color_orientation_animation_and_malformed_time_are_not_silently_excluded() {
        let original = png(image::ExtendedColorType::Rgb8);
        for kind in [b"iCCP", b"eXIf", b"acTL"] {
            assert_eq!(
                inspect(
                    &insert_chunk(&original, kind, &[]),
                    DecodeMode::Pixels,
                    FormatHint::Image,
                    true
                )
                .state,
                ContentState::Unsupported
            );
        }
        assert_eq!(
            inspect(
                &insert_chunk(&original, b"acTL", &[]),
                DecodeMode::Deep,
                FormatHint::Image,
                true
            )
            .state,
            ContentState::Unsupported
        );
        for data in [
            &[0u8; 7][..],
            &[7, 234, 13, 1, 0, 0, 0][..],
            &[7, 234, 2, 30, 0, 0, 0][..],
        ] {
            assert_eq!(
                inspect(
                    &insert_chunk(&original, b"tIME", data),
                    DecodeMode::Pixels,
                    FormatHint::Image,
                    true
                )
                .state,
                ContentState::Unsupported
            );
        }
        let dated = insert_chunk(&original, b"tIME", &[7, 234, 11, 1, 12, 30, 0]);
        assert_eq!(
            inspect(&original, DecodeMode::Pixels, FormatHint::Image, true).pixel_digest,
            inspect(&dated, DecodeMode::Pixels, FormatHint::Image, true).pixel_digest
        );
    }
}

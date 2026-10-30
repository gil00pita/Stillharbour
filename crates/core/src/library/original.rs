//! Bounded original-byte delivery, independent of the derived-preview cache.
use super::*;
use std::io::{Seek, SeekFrom};

pub const ORIGINAL_IMAGE_POLICY: &str = "original-v2-static-heic-32mib-32mp";
const MAX_PIXELS: u64 = 32_000_000;

#[derive(Debug)]
pub struct OriginalImage {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}
struct ActiveRead<'a>(&'a Mutex<Option<Arc<AtomicBool>>>);
impl Drop for ActiveRead<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.lock() {
            *active = None;
        }
    }
}
fn unsupported() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "This original requires a preview fallback (supported static images, at most 32 MiB and 32 megapixels; HEIC requires a compatible macOS webview).",
    )
}
// Refuse animated containers before passing their bytes to the WebView. The
// decoder child independently validates source geometry and decoding policy.
fn static_mime(bytes: &[u8]) -> Result<&'static str> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok("image/jpeg");
    }
    #[cfg(target_os = "macos")]
    if bytes.get(4..8) == Some(b"ftyp") {
        return static_heic(bytes);
    }
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let webp = bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    if !png && !webp {
        return Err(unsupported());
    }
    let mut offset = if png { 8usize } else { 12usize };
    let mut count = 0;
    while offset < bytes.len() {
        count += 1;
        if count > 4096 || bytes.len() - offset < if png { 12 } else { 8 } {
            return Err(unsupported());
        }
        let header = &bytes[offset..offset + 8];
        let (kind, length) = if png {
            (
                &header[4..8],
                u32::from_be_bytes(header[..4].try_into().map_err(|_| unsupported())?) as usize,
            )
        } else {
            (
                &header[..4],
                u32::from_le_bytes(header[4..8].try_into().map_err(|_| unsupported())?) as usize,
            )
        };
        if kind == b"acTL" || kind == b"ANIM" || kind == b"ANMF" {
            return Err(unsupported());
        }
        if kind == b"VP8X" && bytes.get(offset + 8).is_some_and(|flags| flags & 2 != 0) {
            return Err(unsupported());
        }
        let padding = if png { 12 } else { 8 + length % 2 };
        offset = offset
            .checked_add(length)
            .and_then(|v| v.checked_add(padding))
            .filter(|v| *v <= bytes.len())
            .ok_or_else(unsupported)?;
        if png && kind == b"IEND" {
            break;
        }
    }
    Ok(if png { "image/png" } else { "image/webp" })
}
// HEIC still-image brands, not filename suffixes, admit the original bytes.
// Auxiliary images/collections are allowed; timed tracks and sequence brands
// are not. The existing supervised ImageIO worker validates the primary image.
#[cfg(target_os = "macos")]
fn static_heic(bytes: &[u8]) -> Result<&'static str> {
    let length = u32::from_be_bytes(
        bytes
            .get(..4)
            .ok_or_else(unsupported)?
            .try_into()
            .map_err(|_| unsupported())?,
    ) as usize;
    if !(16..=4096).contains(&length) || length > bytes.len() || !length.is_multiple_of(4) {
        return Err(unsupported());
    }
    let major: &[u8; 4] = bytes[8..12].try_into().map_err(|_| unsupported())?;
    if !matches!(major, b"heic" | b"heix" | b"mif1") {
        return Err(unsupported());
    }
    let (compatible, _) = bytes[16..length].as_chunks::<4>();
    let brands = std::iter::once(major).chain(compatible.iter());
    let mut heic = false;
    for brand in brands {
        if matches!(
            brand,
            b"msf1" | b"hevc" | b"hevx" | b"hevm" | b"hevs" | b"avis"
        ) {
            return Err(unsupported());
        }
        heic |= matches!(brand, b"heic" | b"heix");
    }
    if !heic {
        return Err(unsupported());
    }
    let mut offset = length;
    let mut count = 1;
    while offset < bytes.len() {
        count += 1;
        let header = bytes.get(offset..offset + 8).ok_or_else(unsupported)?;
        if count > 4096 || matches!(&header[4..8], b"moov" | b"moof" | b"trak" | b"ftyp") {
            return Err(unsupported());
        }
        let (length, minimum) =
            match u32::from_be_bytes(header[..4].try_into().map_err(|_| unsupported())?) {
                0 => (bytes.len() - offset, 8),
                1 => {
                    let size = bytes.get(offset + 8..offset + 16).ok_or_else(unsupported)?;
                    (
                        usize::try_from(u64::from_be_bytes(
                            size.try_into().map_err(|_| unsupported())?,
                        ))
                        .map_err(|_| unsupported())?,
                        16,
                    )
                }
                size => (size as usize, 8),
            };
        if length < minimum {
            return Err(unsupported());
        }
        offset = offset
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(unsupported)?;
    }
    Ok("image/heic")
}
impl Library {
    /// Main-webview protocol adapter only. IDs/revision never act as paths.
    /// One admitted original read; existing source/CPU/helper limits still apply.
    pub fn original_image(
        &self,
        asset_id: String,
        revision: String,
        policy: &str,
    ) -> Result<OriginalImage> {
        if policy != ORIGINAL_IMAGE_POLICY
            || uuid::Uuid::parse_str(&asset_id).is_err()
            || revision.len() > 256
        {
            return Err(invalid("Invalid original-image identity or policy."));
        }
        let _admission = self.original_gate.try_start()?;
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if self.maintenance.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Library maintenance is running.",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *self.original_cancel.lock().map_err(|_| lock_error())? = Some(cancel.clone());
        let _active = ActiveRead(&self.original_cancel);
        let asset = store::asset(&self.catalog, asset_id.clone())?;
        let (root_id, relative, expected) = store::asset_native(&self.catalog, asset_id.clone())?;
        if expected != revision || asset.revision != revision {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The original changed. Refresh this view.",
            ));
        }
        let root = store::root(&self.catalog, root_id)?;
        if asset.presence != "present"
            || !matches!(root.view.availability.as_str(), "online" | "scanning")
            || asset.metadata_state == "placeholder"
        {
            return Err(AppError::new(
                ErrorCode::Io,
                "Reconnect or download the original to view it.",
            ));
        }
        let mut file = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        if file.metadata()?.len() > crate::media::MAX_INPUT_BYTES {
            return Err(unsupported());
        }
        let _lease = self.resources.acquire(
            filesystem::identity(&file)?.volume,
            Class::Interactive,
            Work::Rendition,
            &cancel,
        )?;
        if native::revision(&file)? != expected {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The original changed. Refresh this view.",
            ));
        }
        // Read through the existing cancellable, 32-MiB, revision-checked adapter.
        let bytes = workers::read_input_cancellable(file.try_clone()?, &cancel)?;
        let mime = static_mime(&bytes)?;
        file.seek(SeekFrom::Start(0))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let report = runtime.block_on(workers::thumbnail_file(
            &self.options.workers,
            file,
            32,
            crate::media::PreviewFormat::Auto,
            cancel.clone(),
        ))?;
        crate::rendition::validate(&report, 32)?;
        if u64::from(report.source_width) * u64::from(report.source_height) > MAX_PIXELS {
            return Err(unsupported());
        }
        let current = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Original-image read canceled.",
            ));
        }
        if native::revision(&current)? != expected
            || store::asset_native(&self.catalog, asset_id)?.2 != expected
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The original changed while loading.",
            ));
        }
        Ok(OriginalImage { bytes, mime })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    fn heic_header(major: &[u8; 4], compatible: &[u8]) -> Vec<u8> {
        let mut bytes = ((16 + compatible.len()) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(b"ftyp");
        bytes.extend_from_slice(major);
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(compatible);
        bytes
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn heic_mime_admits_still_brands_and_bounded_boxes() {
        for (major, compatible) in [
            (b"heic", &b"mif1"[..]),
            (b"heix", &b"mif1"[..]),
            (b"mif1", &b"heic"[..]),
        ] {
            let mut bytes = heic_header(major, compatible);
            assert_eq!(static_mime(&bytes).unwrap(), "image/heic");
            bytes.extend_from_slice(b"\0\0\0\x01mdat\0\0\0\0\0\0\0\x10");
            assert_eq!(static_mime(&bytes).unwrap(), "image/heic");
            bytes.extend_from_slice(b"\0\0\0\0mdatpayload");
            assert_eq!(static_mime(&bytes).unwrap(), "image/heic");
        }
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn heic_mime_rejects_sequences_malformed_bounds_and_other_codecs() {
        let valid = heic_header(b"heic", b"mif1");
        for end in 0..valid.len() {
            assert!(static_mime(&valid[..end]).is_err());
        }
        for major in [b"avif", b"msf1", b"hevc", b"hevx"] {
            assert!(static_mime(&heic_header(major, b"heic")).is_err());
        }
        for brand in [b"msf1", b"hevc", b"hevx", b"hevm", b"hevs", b"avis"] {
            assert!(static_mime(&heic_header(b"heic", brand)).is_err());
        }
        let mut minor_only = heic_header(b"mif1", b"");
        minor_only[12..16].copy_from_slice(b"heic");
        assert!(static_mime(&minor_only).is_err());
        for suffix in [
            &b"\0\0\0\x08moov"[..],
            &b"\0\0\0\x08moof"[..],
            &b"\0\0\0\x08trak"[..],
            &b"\0\0\0\x07free"[..],
            &b"\0\0\0\xffmdat"[..],
            &b"\0\0\0\x01mdat"[..],
            &b"\0\0\0\x01mdat\xff\xff\xff\xff\xff\xff\xff\xff"[..],
        ] {
            let mut bytes = valid.clone();
            bytes.extend_from_slice(suffix);
            assert!(static_mime(&bytes).is_err());
        }
        assert!(static_mime(&heic_header(b"heic", b"unaligned")).is_err());
        assert!(static_mime(&heic_header(b"heic", &vec![0; 4096])).is_err());
        let mut excessive = valid;
        for _ in 0..4096 {
            excessive.extend_from_slice(b"\0\0\0\x08free");
        }
        assert!(static_mime(&excessive).is_err());
    }
    #[test]
    fn original_mime_rejects_active_animated_and_truncated_containers() {
        assert!(static_mime(b"<svg onload='active' />").is_err());
        assert!(static_mime(b"\x89PNG\r\n\x1a\n\0\0\0\x08acTL\0\0\0\x02\0\0\0\0xxxx").is_err());
        assert!(static_mime(b"RIFFxxxxWEBPANIM\0\0\0\0").is_err());
        assert!(static_mime(b"\x89PNG\r\n\x1a\n\xff\xff\xff\xffIDATxxxx").is_err());
    }
}

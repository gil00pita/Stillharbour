//! Bounded original-byte delivery, independent of the derived-preview cache.
use super::*;
use std::io::{Seek, SeekFrom};

pub const ORIGINAL_IMAGE_POLICY: &str = "original-v1-static-32mib-32mp";
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
        "This original requires a preview fallback (static JPEG/PNG/WebP, at most 32 MiB and 32 megapixels).",
    )
}
// Refuse animated containers before passing their bytes to the WebView. The
// decoder child independently validates source geometry and decoding policy.
fn static_mime(bytes: &[u8]) -> Result<&'static str> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok("image/jpeg");
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
    #[test]
    fn original_mime_rejects_active_animated_and_truncated_containers() {
        assert!(static_mime(b"<svg onload='active' />").is_err());
        assert!(static_mime(b"\x89PNG\r\n\x1a\n\0\0\0\x08acTL\0\0\0\x02\0\0\0\0xxxx").is_err());
        assert!(static_mime(b"RIFFxxxxWEBPANIM\0\0\0\0").is_err());
        assert!(static_mime(b"\x89PNG\r\n\x1a\n\xff\xff\xff\xffIDATxxxx").is_err());
    }
}

//! Generated media only, excluded from production builds. No caller-selected paths.
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct LibraryFixture {
    directory: tempfile::TempDir,
    baseline: BTreeMap<PathBuf, String>,
}
impl LibraryFixture {
    pub fn generate() -> Result<Self> {
        Self::generate_in(None)
    }
    pub fn generate_in(parent: Option<&Path>) -> Result<Self> {
        let mut builder = tempfile::Builder::new();
        builder.prefix("stillharbour-generated-");
        let directory = if let Some(parent) = parent {
            builder.tempdir_in(parent)?
        } else {
            builder.tempdir()?
        };
        let root = directory.path();
        for folder in ["Coast", "Woodland", ".hidden", "Private", "Application.app"] {
            std::fs::create_dir(root.join(folder))?;
        }
        for index in 0..18u32 {
            let (width, height) = if index % 3 == 0 {
                (360, 480)
            } else {
                (600, 380)
            };
            let image = image::RgbImage::from_fn(width, height, |x, y| {
                let sky = y < height * 5 / 9;
                let seed = index * 11;
                if sky {
                    image::Rgb([
                        ((70 + x / 8 + seed) % 190) as u8,
                        (125 + y / 6) as u8,
                        (155 + x / 12) as u8,
                    ])
                } else {
                    image::Rgb([
                        ((30 + seed + y / 4) % 200) as u8,
                        ((90 + x / 10 + seed) % 210) as u8,
                        ((100 + y / 7 + seed) % 220) as u8,
                    ])
                }
            });
            let extension = ["jpg", "png", "webp"][(index % 3) as usize];
            let folder = if index % 2 == 0 { "Coast" } else { "Woodland" };
            image
                .save(root.join(format!("{folder}/Generated-{index:02}.{extension}")))
                .map_err(|e| {
                    AppError::new(ErrorCode::WorkerFailed, "Could not generate test media.")
                        .caused_by(e)
                })?;
        }
        // A real EXIF IFD supplies pre-1970 capture time, subseconds and explicit offset.
        use image::ImageEncoder;
        let mut tiff = b"II\x2a\0\x08\0\0\0".to_vec();
        tiff.extend_from_slice(&1u16.to_le_bytes());
        fn entry(bytes: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32) {
            bytes.extend(tag.to_le_bytes());
            bytes.extend(kind.to_le_bytes());
            bytes.extend(count.to_le_bytes());
            bytes.extend(value.to_le_bytes());
        }
        entry(&mut tiff, 0x8769, 4, 1, 26);
        tiff.extend(0u32.to_le_bytes());
        tiff.extend(3u16.to_le_bytes());
        entry(&mut tiff, 0x9003, 2, 20, 68);
        entry(&mut tiff, 0x9011, 2, 7, 88);
        entry(&mut tiff, 0x9291, 2, 5, 95);
        tiff.extend(0u32.to_le_bytes());
        tiff.extend(b"1964:02:29 13:15:20\0+05:30\x001234\0");
        let image = image::RgbImage::from_fn(600, 380, |x, y| {
            image::Rgb([(45 + x / 7) as u8, (90 + y / 4) as u8, (110 + x / 9) as u8])
        });
        let mut encoded = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut encoded);
        encoder.set_exif_metadata(tiff).map_err(|e| {
            AppError::new(ErrorCode::WorkerFailed, "EXIF fixture failed.").caused_by(e)
        })?;
        encoder
            .write_image(image.as_raw(), 600, 380, image::ExtendedColorType::Rgb8)
            .map_err(|e| {
                AppError::new(ErrorCode::WorkerFailed, "JPEG fixture failed.").caused_by(e)
            })?;
        std::fs::write(root.join("Coast/Generated-00.jpg"), encoded)?;
        std::fs::rename(
            root.join("Woodland/Generated-17.webp"),
            root.join("Woodland/-Generated-17-\"%\n港📷.webp"),
        )?;
        for name in [
            ".hidden.png",
            ".hidden/nested.png",
            "Private/excluded.png",
            "Application.app/protected.png",
        ] {
            image::RgbImage::from_pixel(24, 16, image::Rgb([30, 90, 70]))
                .save(root.join(name))
                .map_err(|e| {
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "Could not generate hidden fixture.",
                    )
                    .caused_by(e)
                })?;
        }
        std::fs::write(
            root.join("Coast/Generated-00.xmp"),
            b"<fixture>companion unchanged</fixture>",
        )?;
        std::fs::write(
            root.join("unsupported.heic"),
            b"generated unsupported fixture",
        )?;
        std::fs::write(root.join("broken.jpg"), b"generated malformed fixture")?;
        std::fs::write(
            root.join("-quoted\"%\n港.txt"),
            b"untrusted filename fixture",
        )?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(root, root.join("loop"))?;
        let baseline = manifest(root)?;
        Ok(Self {
            directory,
            baseline,
        })
    }
    pub fn generate_native_formats() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let mut fixture = Self::generate()?;
            let source = fixture.path().join("Coast/Generated-00.jpg");
            for (name, format) in [
                ("Native HEIC.heic", "heic"),
                ("Native TIFF.tif", "tiff"),
                ("Native GIF.gif", "gif"),
                ("Native BMP.bmp", "bmp"),
            ] {
                let status = std::process::Command::new("/usr/bin/sips")
                    .args(["-s", "format", format])
                    .arg(&source)
                    .arg("--out")
                    .arg(fixture.path().join(name))
                    .env_clear()
                    .current_dir(fixture.path())
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?;
                if !status.success() {
                    return Err(AppError::new(
                        ErrorCode::WorkerFailed,
                        "Native fixture encoding failed.",
                    ));
                }
            }
            // These are fixture construction writes, before the read-only baseline.
            fixture.baseline = manifest(fixture.path())?;
            Ok(fixture)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(AppError::new(
                ErrorCode::Unsupported,
                "Native format fixture creation is unverified on this platform.",
            ))
        }
    }
    pub fn retain(self) -> PathBuf {
        self.directory.keep()
    }
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
    pub fn verify_unchanged(&self) -> Result<u32> {
        if manifest(self.path())? != self.baseline {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Generated source names or bytes changed.",
            ));
        }
        Ok(self.baseline.len() as u32)
    }
}
fn manifest(root: &Path) -> Result<BTreeMap<PathBuf, String>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, String>) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            let relative = path
                .strip_prefix(root)
                .map_err(|e| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Fixture escaped its owned directory.",
                    )
                    .caused_by(e)
                })?
                .to_owned();
            if kind.is_dir() {
                out.insert(relative, "directory".into());
                visit(root, &path, out)?;
            } else if kind.is_symlink() {
                out.insert(relative, format!("link:{:?}", std::fs::read_link(&path)?));
            } else {
                out.insert(
                    relative,
                    blake3::hash(&std::fs::read(path)?).to_hex().to_string(),
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out)?;
    Ok(out)
}

/// Test-only synthetic catalog observations, not a filesystem scan or preview
/// fixture. Rows explicitly have no original; use Text list for paging checks.
pub fn seed_scroll_catalog(
    catalog: &crate::catalog::Catalog,
    fixture: &LibraryFixture,
) -> Result<u32> {
    fixture.verify_unchanged()?;
    let path = crate::library::native::encode_path(&fixture.path().canonicalize()?);
    catalog.call(move |db| {
        let root: String = db.query_row("SELECT id FROM roots WHERE native_path=?", [path], |r|r.get(0))?;
        let tx=db.transaction()?;
        for index in 0..960 {
            let relative=format!("__catalog_fixture__/Synthetic {index:04}.jpg");
            tx.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,preview_error,presence,is_media,seen_scan) VALUES(?1,?2,?3,?4,?5,'__catalog_fixture__',?6,?6,'jpg',?1,1000,'synthetic','2026-10-28T00:00:00Z','2026-10-28T00:00:00','fixture','unavailable','unavailable','Synthetic catalog fixture; no original file','placeholder',1,'synthetic')",rusqlite::params![uuid::Uuid::new_v4().to_string(),root,crate::library::native::encode_path(Path::new(&relative)),relative,crate::library::native::encode_path(Path::new("__catalog_fixture__")),format!("Synthetic {index:04}.jpg")])?;
        }
        tx.execute("UPDATE library_state SET revision=revision+1",[])?;
        tx.commit()?; Ok(960)
    })
}

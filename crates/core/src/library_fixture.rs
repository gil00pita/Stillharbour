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
    /// Closed GPS fixture with real EXIF references; no supplied paths or bytes.
    pub fn generate_geographic() -> Result<Self> {
        use image::ImageEncoder;
        let directory = tempfile::Builder::new()
            .prefix("stillharbour-generated-map-")
            .tempdir()?;
        for (index, (name, point)) in [
            ("Zero", Some((0.0_f64, 0.0_f64))),
            ("Same", Some((0.0, 0.0))),
            ("East", Some((5.0, 179.75))),
            ("West", Some((5.0, -179.75))),
            ("London", Some((51.5074, -0.1278))),
            ("South", Some((-33.5, -70.7))),
            ("Invalid", Some((91.0, 10.0))),
            ("Unknown", None),
        ]
        .into_iter()
        .enumerate()
        {
            let image = image::RgbImage::from_fn(360, 240, |x, y| {
                image::Rgb([
                    (40 + x / 3 + index as u32 * 9) as u8,
                    (70 + y / 2) as u8,
                    125,
                ])
            });
            let mut encoded = Vec::new();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut encoded);
            if let Some((latitude, longitude)) = point {
                let mut tiff = b"II\x2a\0\x08\0\0\0".to_vec();
                fn entry(out: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32) {
                    out.extend(tag.to_le_bytes());
                    out.extend(kind.to_le_bytes());
                    out.extend(count.to_le_bytes());
                    out.extend(value.to_le_bytes());
                }
                tiff.extend(1u16.to_le_bytes());
                entry(&mut tiff, 0x8825, 4, 1, 26);
                tiff.extend(0u32.to_le_bytes());
                tiff.extend(4u16.to_le_bytes());
                entry(
                    &mut tiff,
                    1,
                    2,
                    2,
                    if latitude < 0.0 {
                        b'S'.into()
                    } else {
                        b'N'.into()
                    },
                );
                entry(&mut tiff, 2, 5, 3, 80);
                entry(
                    &mut tiff,
                    3,
                    2,
                    2,
                    if longitude < 0.0 {
                        b'W'.into()
                    } else {
                        b'E'.into()
                    },
                );
                entry(&mut tiff, 4, 5, 3, 104);
                tiff.extend(0u32.to_le_bytes());
                for coordinate in [latitude, longitude] {
                    let degrees = coordinate.abs().floor();
                    let minutes = ((coordinate.abs() - degrees) * 60.0).floor();
                    let seconds = ((coordinate.abs() - degrees) * 60.0 - minutes) * 60.0;
                    for (numerator, denominator) in [
                        (degrees as u32, 1_u32),
                        (minutes as u32, 1_u32),
                        ((seconds * 1_000_000.0).round() as u32, 1_000_000_u32),
                    ] {
                        tiff.extend(numerator.to_le_bytes());
                        tiff.extend(denominator.to_le_bytes());
                    }
                }
                encoder.set_exif_metadata(tiff).map_err(|e| {
                    AppError::new(ErrorCode::WorkerFailed, "GPS fixture EXIF failed.").caused_by(e)
                })?;
            }
            encoder
                .write_image(image.as_raw(), 360, 240, image::ExtendedColorType::Rgb8)
                .map_err(|e| {
                    AppError::new(ErrorCode::WorkerFailed, "GPS fixture JPEG failed.").caused_by(e)
                })?;
            std::fs::write(directory.path().join(format!("{name}.jpg")), encoded)?;
        }
        std::fs::write(
            directory.path().join("Zero.xmp"),
            br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Description xmlns:p="urn:stillharbour:fixture:unknown" p:EditingHistory="Retain this unknown field"/></r:RDF></x:xmpmeta>"#,
        )?;
        let baseline = manifest(directory.path())?;
        Ok(Self {
            directory,
            baseline,
        })
    }
    /// Only the constructor-owned valid JPEG and its companion; unlike the
    /// malformed-format coverage fixture, this supports a complete real scan.
    pub fn coherent_import_copy(&self) -> Result<Self> {
        use std::io::Write;
        self.verify_unchanged()?;
        let directory = tempfile::Builder::new()
            .prefix("stillharbour-complete-import-")
            .tempdir()?;
        for (source, target) in [
            ("Coast/Generated-00.jpg", "Renamed.jpg"),
            ("Coast/Generated-00.xmp", "Renamed.xmp"),
        ] {
            let relative = Path::new(source);
            if !self.baseline.contains_key(relative) {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "The expected generated import member is missing.",
                ));
            }
            let path = self.path().join(relative);
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Unexpected generated import member type or size.",
                ));
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.path().join(target))?;
            output.write_all(&std::fs::read(path)?)?;
        }
        self.verify_unchanged()?;
        let baseline = manifest(directory.path())?;
        Ok(Self {
            directory,
            baseline,
        })
    }
    /// Constructor-only copies of this generated baseline, with different
    /// filenames. No renderer-supplied root, filename or content is accepted.
    pub fn renamed_comparison_copy(&self) -> Result<Self> {
        use std::io::Write;
        self.verify_unchanged()?;
        let directory = tempfile::Builder::new()
            .prefix("stillharbour-import-comparison-")
            .tempdir()?;
        for (index, (relative, signature)) in self.baseline.iter().enumerate() {
            if relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated comparison source escaped its boundary.",
                ));
            }
            if signature == "directory" || signature.starts_with("link:") {
                continue;
            }
            let source = self.path().join(relative);
            if !std::fs::symlink_metadata(&source)?.is_file()
                || source.metadata()?.len() > 16 * 1024 * 1024
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Unexpected generated comparison source type or size.",
                ));
            }
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            let mut name = std::ffi::OsString::from(format!("existing-{index:04}-"));
            name.push(relative.file_name().ok_or_else(|| {
                AppError::new(ErrorCode::Conflict, "Missing generated filename.")
            })?);
            let target_parent = directory.path().join(parent);
            std::fs::create_dir_all(&target_parent)?;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target_parent.join(name))?;
            file.write_all(&std::fs::read(source)?)?;
        }
        self.verify_unchanged()?;
        let baseline = manifest(directory.path())?;
        Ok(Self {
            directory,
            baseline,
        })
    }
    /// Separate generated keyword fixture; never changes a caller-provided path.
    pub fn generate_tagged() -> Result<Self> {
        let mut fixture = Self::generate()?;
        for (name, labels) in [
            (
                "Coast/Generated-00.jpg",
                "<rdf:li>Coast</rdf:li><rdf:li>Outdoors</rdf:li>",
            ),
            ("Coast/Generated-06.jpg", "<rdf:li>Outdoors</rdf:li>"),
        ] {
            let path = fixture.path().join(name);
            let original = std::fs::read(&path)?;
            let xml = format!(
                r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:subject><rdf:Bag>{labels}</rdf:Bag></dc:subject></rdf:Description></rdf:RDF></x:xmpmeta>"#
            );
            let mut packet = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
            packet.extend(xml.as_bytes());
            let mut tagged = vec![0xff, 0xd8, 0xff, 0xe1];
            tagged.extend(((packet.len() + 2) as u16).to_be_bytes());
            tagged.extend(packet);
            tagged.extend(&original[2..]);
            std::fs::write(path, tagged)?;
        }
        fixture.baseline = manifest(fixture.path())?;
        Ok(fixture)
    }
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
        // Sensor-landscape JPEG displayed as portrait by EXIF orientation 6.
        // Replace an existing generated item so inventory counts stay stable.
        let mut rotated_exif = b"II\x2a\0\x08\0\0\0".to_vec();
        rotated_exif.extend(1u16.to_le_bytes());
        entry(&mut rotated_exif, 0x0112, 3, 1, 6);
        rotated_exif.extend(0u32.to_le_bytes());
        let image = image::RgbImage::from_fn(600, 450, |x, y| {
            image::Rgb([(40 + x / 8) as u8, (70 + y / 5) as u8, 130])
        });
        let mut encoded = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut encoded);
        encoder.set_exif_metadata(rotated_exif).map_err(|e| {
            AppError::new(ErrorCode::WorkerFailed, "Rotated EXIF fixture failed.").caused_by(e)
        })?;
        encoder
            .write_image(image.as_raw(), 600, 450, image::ExtendedColorType::Rgb8)
            .map_err(|e| {
                AppError::new(ErrorCode::WorkerFailed, "Rotated JPEG fixture failed.").caused_by(e)
            })?;
        std::fs::write(root.join("Coast/Generated-06.jpg"), encoded)?;
        std::fs::rename(
            root.join("Woodland/Generated-17.webp"),
            root.join(if cfg!(windows) {
                "Woodland/-Generated-17-%港📷.webp"
            } else {
                "Woodland/-Generated-17-\"%\n港📷.webp"
            }),
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
            root.join(if cfg!(windows) {
                "-quoted%港.txt"
            } else {
                "-quoted\"%\n港.txt"
            }),
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
            let encoding = tempfile::tempdir()?;
            let full_size = encoding.path().join("Generated.png");
            image::RgbImage::from_fn(3072, 2048, |x, y| {
                image::Rgb([(x / 16 % 240) as u8, (y / 12 % 240) as u8, 120])
            })
            .save(&full_size)
            .map_err(|error| {
                AppError::new(ErrorCode::WorkerFailed, "Could not generate HEIC source.")
                    .caused_by(error)
            })?;
            for (name, format) in [
                ("Native HEIC.heic", "heic"),
                ("Native TIFF.tif", "tiff"),
                ("Native GIF.gif", "gif"),
                ("Native BMP.bmp", "bmp"),
            ] {
                let status = std::process::Command::new("/usr/bin/sips")
                    .args(["-s", "format", format])
                    .arg(if format == "heic" {
                        &full_size
                    } else {
                        &source
                    })
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
    /// Test-host caller supplies only its own constructor-created destination.
    /// This independent byte audit accepts partial stages without claiming they
    /// are completed copies. It never modifies or cleans an output object.
    pub fn audit_generated_outputs(&self, destination: &Path) -> Result<serde_json::Value> {
        let verified = self.verify_unchanged()?;
        if std::fs::read(destination.join("destination-only.txt"))? != b"retained destination" {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Generated destination-only bytes changed.",
            ));
        }
        fn visit(
            fixture: &LibraryFixture,
            root: &Path,
            path: &Path,
            depth: usize,
            files: &mut Vec<serde_json::Value>,
            count: &mut usize,
        ) -> Result<()> {
            if depth > 32 {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated output audit exceeded depth bound.",
                ));
            }
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                *count += 1;
                if *count > 4096 {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Generated output audit exceeded entry bound.",
                    ));
                }
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| {
                        AppError::new(
                            ErrorCode::Conflict,
                            "Generated output escaped its boundary.",
                        )
                    })?
                    .to_owned();
                if relative == Path::new("destination-only.txt") {
                    continue;
                }
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Unexpected link in generated output.",
                    ));
                }
                if kind.is_dir() {
                    visit(fixture, root, &entry.path(), depth + 1, files, count)?;
                } else if kind.is_file() && entry.metadata()?.len() <= 16 * 1024 * 1024 {
                    let bytes = std::fs::read(entry.path())?;
                    let digest = blake3::hash(&bytes).to_hex().to_string();
                    let mut matches = false;
                    for (source, baseline) in &fixture.baseline {
                        if baseline.split(':').next() == Some(digest.as_str())
                            && std::fs::symlink_metadata(fixture.path().join(source))?.is_file()
                            && std::fs::read(fixture.path().join(source))? == bytes
                        {
                            matches = true;
                            break;
                        }
                    }
                    files.push(serde_json::json!({"relativePath":relative.to_string_lossy(),"bytes":bytes.len().to_string(),"digest":digest,"matchesGeneratedSourceBytes":matches}));
                } else {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Unexpected generated output type or size.",
                    ));
                }
            }
            Ok(())
        }
        let mut files = Vec::new();
        let mut count = 0;
        visit(self, destination, destination, 0, &mut files, &mut count)?;
        Ok(
            serde_json::json!({"sourceVerifiedFiles":verified,"neighborUnchanged":true,"files":files,"auditedEntries":count,"writesPerformed":false}),
        )
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
                let hash = blake3::hash(&std::fs::read(&path)?).to_hex().to_string();
                #[cfg(windows)]
                let hash = {
                    use std::os::windows::fs::MetadataExt;
                    let m = std::fs::metadata(&path)?;
                    format!(
                        "{hash}:{}:{}:{}:{}",
                        m.len(),
                        m.creation_time(),
                        m.last_write_time(),
                        m.file_attributes()
                    )
                };
                out.insert(relative, hash);
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

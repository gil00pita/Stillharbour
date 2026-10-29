use super::model::ScanFilter;
use crate::domain::{AppError, ErrorCode, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::path::Path;

pub struct CompiledFilter {
    pub spec: ScanFilter,
    includes: GlobSet,
    excludes: GlobSet,
    min: Option<u64>,
    max: Option<u64>,
}
pub const LOCKED_FOLDERS: &[&str] = &[
    ".git",
    "node_modules",
    "@eaDir",
    ".Trashes",
    ".Spotlight-V100",
    ".fseventsd",
    ".TemporaryItems",
];

fn compile(patterns: &[String]) -> Result<GlobSet> {
    if patterns.len() > 32 {
        return Err(invalid("Use at most 32 patterns per filter."));
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if pattern.len() > 256
            || pattern.contains('\0')
            || pattern.starts_with('/')
            || pattern.split('/').any(|p| p == "..")
        {
            return Err(invalid("Patterns must be bounded root-relative globs."));
        }
        builder.add(
            GlobBuilder::new(pattern)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|e| invalid("Invalid glob pattern.").caused_by(e))?,
        );
    }
    builder
        .build()
        .map_err(|e| invalid("Filter is too complex.").caused_by(e))
}

pub fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
pub fn decimal(value: &str) -> Result<Option<u64>> {
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| invalid("Size must be a nonnegative decimal byte count."))
}
pub fn validate_date(value: &str) -> Result<()> {
    if !value.is_empty() && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(invalid("Dates must use YYYY-MM-DD."));
    }
    Ok(())
}
impl CompiledFilter {
    pub fn new(spec: ScanFilter) -> Result<Self> {
        if spec.version != 1 || spec.excluded_folders.len() > 32 || spec.extensions.len() > 64 {
            return Err(invalid("Unsupported or oversized scan filter."));
        }
        for name in &spec.excluded_folders {
            if name.len() > 128 || name.contains(['/', '\\', '\0']) || name == ".." {
                return Err(invalid("Excluded folders must be basenames."));
            }
        }
        for ext in &spec.extensions {
            if ext.len() > 16 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(invalid("Extensions must contain letters and digits only."));
            }
        }
        let min = decimal(spec.min_bytes.as_deref().unwrap_or_default())?;
        let max = decimal(spec.max_bytes.as_deref().unwrap_or_default())?;
        if min.zip(max).is_some_and(|(a, b)| a > b) {
            return Err(invalid("Minimum size exceeds maximum size."));
        }
        validate_date(spec.from_date.as_deref().unwrap_or_default())?;
        validate_date(spec.through_date.as_deref().unwrap_or_default())?;
        if spec
            .from_date
            .as_ref()
            .zip(spec.through_date.as_ref())
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a > b)
        {
            return Err(invalid("Start date exceeds end date."));
        }
        Ok(Self {
            includes: compile(&spec.include_globs)?,
            excludes: compile(&spec.exclude_globs)?,
            spec,
            min,
            max,
        })
    }
    pub fn directory_reason(&self, path: &Path, hidden: bool) -> Option<&'static str> {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if LOCKED_FOLDERS.contains(&name.as_ref())
            || name.ends_with(".photoslibrary")
            || name.ends_with(".app")
            || name.ends_with(".bundle")
        {
            return Some("Locked system/package exclusion");
        }
        if hidden && !self.spec.include_hidden_folders {
            return Some("Hidden folder excluded");
        }
        if self
            .spec
            .excluded_folders
            .iter()
            .any(|n| n == name.as_ref())
        {
            return Some("Folder basename excluded");
        }
        if self.excludes.is_match(path) || self.excludes.is_match(path.join("__entry__")) {
            return Some("Excluded by path glob");
        }
        None
    }
    pub fn file_reason(
        &self,
        path: &Path,
        hidden: bool,
        bytes: u64,
        date: Option<&str>,
    ) -> Option<&'static str> {
        if path.file_name().is_some_and(|n| n == ".DS_Store") {
            return Some("Locked system exclusion");
        }
        if hidden && !self.spec.include_hidden_files {
            return Some("Hidden file excluded");
        }
        if self.excludes.is_match(path) {
            return Some("Excluded by path glob");
        }
        if !self.spec.include_globs.is_empty() && !self.includes.is_match(path) {
            return Some("No include pattern matched");
        }
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        if !self.spec.extensions.is_empty()
            && !self
                .spec
                .extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&ext))
        {
            return Some("Extension excluded");
        }
        if self.min.is_some_and(|min| bytes < min) || self.max.is_some_and(|max| bytes > max) {
            return Some("Outside size range");
        }
        if let Some(date) = date
            && (self
                .spec
                .from_date
                .as_deref()
                .is_some_and(|from| !from.is_empty() && date < from)
                || self
                    .spec
                    .through_date
                    .as_deref()
                    .is_some_and(|to| !to.is_empty() && date > to))
        {
            return Some("Outside date range");
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_do_not_prune_parents_and_exclusions_win() {
        let spec = ScanFilter {
            include_globs: vec!["**/chosen/*.jpg".into()],
            exclude_globs: vec!["**/private/**".into()],
            ..Default::default()
        };
        let f = CompiledFilter::new(spec).unwrap();
        assert!(f.directory_reason(Path::new("parent"), false).is_none());
        assert!(
            f.file_reason(Path::new("parent/chosen/photo.jpg"), false, 10, None)
                .is_none()
        );
        assert!(
            f.directory_reason(Path::new("parent/private"), false)
                .is_some()
        );
        assert!(
            f.file_reason(Path::new("parent/private/chosen/a.jpg"), false, 10, None)
                .is_some()
        );
    }
    #[test]
    fn hidden_controls_are_independent_and_inputs_are_bounded() {
        let f = CompiledFilter::new(ScanFilter {
            include_hidden_files: true,
            ..Default::default()
        })
        .unwrap();
        assert!(
            f.file_reason(Path::new(".image.jpg"), true, 1, None)
                .is_none()
        );
        assert!(f.directory_reason(Path::new(".folder"), true).is_some());
        assert!(
            CompiledFilter::new(ScanFilter {
                include_globs: vec!["../escape".into()],
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            f.directory_reason(Path::new("album.photoslibrary"), false)
                .is_some()
        );
    }
    #[test]
    fn photo_preset_matches_upper_and_lower_case_extensions() {
        let filter = CompiledFilter::new(ScanFilter {
            extensions: super::super::model::DEFAULT_PHOTO_EXTENSIONS
                .iter()
                .map(|e| e.to_string())
                .collect(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(filter.spec.extensions.len(), 33);
        for ext in super::super::model::DEFAULT_PHOTO_EXTENSIONS {
            for suffix in [ext.to_string(), ext.to_ascii_uppercase()] {
                assert!(
                    filter
                        .file_reason(Path::new(&format!("photo.{suffix}")), false, 1, None)
                        .is_none(),
                    "{suffix}"
                );
            }
        }
        assert_eq!(
            filter.file_reason(Path::new("document.pdf"), false, 1, None),
            Some("Extension excluded")
        );
    }
}

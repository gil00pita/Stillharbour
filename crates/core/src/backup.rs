//! Reviewed process boundary only. No production backup/restore or shell command API.
use crate::domain::{AppError, ErrorCode, Result};
use std::{ffi::OsString, path::Path};

#[derive(Debug, Clone, Copy)]
pub enum Helper {
    ExifTool,
    Restic,
    Rclone,
}

impl Helper {
    pub fn packaged_stem(self) -> &'static str {
        match self {
            Self::ExifTool => "exiftool",
            Self::Restic => "restic",
            Self::Rclone => "rclone",
        }
    }
}

pub fn metadata_arguments(input: &Path) -> Result<Vec<OsString>> {
    if !input.is_absolute() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Metadata input must be a backend-selected absolute native path.",
        ));
    }
    // -config must be first. Separate argv entries preserve newlines and leading
    // hyphens; never use ExifTool's line-delimited -@ protocol for native filenames.
    Ok(metadata_read_arguments(input.as_os_str()))
}

pub(crate) fn metadata_stdin_arguments() -> Vec<OsString> {
    metadata_read_arguments(std::ffi::OsStr::new("-"))
}
fn metadata_read_arguments(input: &std::ffi::OsStr) -> Vec<OsString> {
    [
        "-config",
        "",
        "-j",
        "-api",
        "structformat=jsonq",
        "-n",
        "-ImageWidth",
        "-ImageHeight",
        "-Orientation",
        "-ColorSpace",
        "-DateTimeOriginal",
        "-SubSecTimeOriginal",
        "-OffsetTimeOriginal",
        "-CreateDate",
        "-SubSecTimeDigitized",
        "-OffsetTimeDigitized",
        "-Make",
        "-Model",
        "-FileType",
        "-Keywords",
        "-Subject",
        "--",
    ]
    .into_iter()
    .map(OsString::from)
    .chain([input.to_owned()])
    .collect()
}

pub fn production_backup_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostile_filename_is_one_argument_and_config_is_disabled_first() {
        let path = std::env::temp_dir().join("-quote\" percent%\n港📷.jpg");
        let args = metadata_arguments(&path).unwrap();
        assert_eq!(args[0], "-config");
        assert_eq!(args[1], "");
        assert_eq!(args[args.len() - 2], "--");
        assert_eq!(args.last().unwrap(), path.as_os_str());
        assert!(!production_backup_available());
    }
}

//! Versioned, bounded essential metadata for ordinary files. Inspection is
//! handle-only and read-only. Unsupported streams/attributes fail explicitly.
//! Restoring metadata is restricted to a different, newly created owned output;
//! the executor must retain its exclusive publication/journal authority.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
    library::native,
};
use serde::{Deserialize, Serialize};
use std::fs::File;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;

const MAX_METADATA: usize = 256 * 1024;
#[cfg(target_os = "macos")]
pub(crate) struct PrivateDirectoryWitness {
    identity: FileIdentity,
    policy: (u32, u32, u32),
}
#[cfg(target_os = "macos")]
pub(crate) fn private_directory(parent: &File) -> Result<PrivateDirectoryWitness> {
    let identity = filesystem::identity(parent)?;
    let revision = native::revision(parent)?;
    let policy = macos::private_directory(parent)?;
    if filesystem::identity(parent)? != identity || native::revision(parent)? != revision {
        return Err(changed());
    }
    Ok(PrivateDirectoryWitness { identity, policy })
}
#[cfg(target_os = "macos")]
impl PrivateDirectoryWitness {
    pub(crate) fn revalidate(&self, parent: &File) -> Result<()> {
        let current = private_directory(parent)?;
        if current.identity.volume != self.identity.volume
            || current.identity.file != self.identity.file
            || current.policy != self.policy
        {
            return Err(changed());
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
pub(crate) fn verify_private_file(file: &File) -> Result<()> {
    let identity = filesystem::identity(file)?;
    let revision = native::revision(file)?;
    macos::private_file(file)?;
    if filesystem::identity(file)? != identity || native::revision(file)? != revision {
        return Err(changed());
    }
    Ok(())
}
fn unsupported(message: &str) -> AppError {
    AppError::new(ErrorCode::Unsupported, message)
}
fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn changed() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "Essential native metadata changed during inspection or differs from the exact recovery manifest.",
    )
}
fn inspect_native(file: &File) -> Result<NativeMetadata> {
    #[cfg(windows)]
    {
        windows::inspect(file)
    }
    #[cfg(target_os = "macos")]
    {
        macos::inspect(file)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = file;
        Err(unsupported("Essential metadata is unavailable on this OS."))
    }
}
fn restore_native(file: &File, metadata: &NativeMetadata) -> Result<()> {
    #[cfg(windows)]
    {
        windows::restore(file, metadata)
    }
    #[cfg(target_os = "macos")]
    {
        macos::restore(file, metadata)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (file, metadata);
        Err(unsupported(
            "Essential metadata restoration is unavailable on this OS.",
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "platform", rename_all = "snake_case", deny_unknown_fields)]
enum NativeMetadata {
    Windows {
        attributes: u32,
        created: String,
        modified: String,
        security_descriptor: Vec<u8>,
        mandatory_label: Vec<u8>,
        audit_descriptor: Option<Vec<u8>>,
    },
    Macos {
        uid: u32,
        gid: u32,
        mode: u32,
        flags: u32,
        created_seconds: i64,
        created_nanoseconds: i64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MetadataManifest {
    version: u32,
    policy: String,
    origin: FileIdentity,
    metadata: NativeMetadata,
}
impl MetadataManifest {
    pub(crate) fn inspect(file: &File) -> Result<Self> {
        let origin = filesystem::identity(file)?;
        let revision = native::revision(file)?;
        if origin.links != 1 || !file.metadata()?.is_file() {
            return Err(unsupported(
                "Essential metadata requires an ordinary single-link native file.",
            ));
        }
        let metadata = inspect_native(file)?;
        let manifest = Self {
            version: 1,
            policy: "essential-no-extra-streams-v1-owner-permissions-creation-modification".into(),
            origin,
            metadata,
        };
        manifest.validate()?;
        if filesystem::identity(file)? != manifest.origin || native::revision(file)? != revision {
            return Err(changed());
        }
        Ok(manifest)
    }
    fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.policy
                != "essential-no-extra-streams-v1-owner-permissions-creation-modification"
            || self.origin.links != 1
        {
            return Err(invalid(
                "Unsupported essential metadata policy or origin binding.",
            ));
        }
        let bytes = serde_json::to_vec(self)
            .map_err(|_| invalid("Invalid essential metadata manifest."))?;
        if bytes.len() > MAX_METADATA {
            return Err(invalid("Essential metadata exceeds its 256 KiB bound."));
        }
        match &self.metadata {
            NativeMetadata::Windows {
                created,
                modified,
                security_descriptor,
                attributes,
                mandatory_label,
                audit_descriptor,
            } => {
                if created.parse::<i64>().is_err()
                    || modified.parse::<i64>().is_err()
                    || security_descriptor.len() > 65536
                    || *attributes & !0x0000_20a7 != 0
                {
                    return Err(invalid("Invalid Windows essential metadata."));
                }
                #[cfg(windows)]
                {
                    windows::validate_descriptor(security_descriptor)?;
                    windows::validate_sacl(mandatory_label)?;
                    if let Some(audit) = audit_descriptor {
                        windows::validate_sacl(audit)?;
                    }
                }
                #[cfg(not(windows))]
                let _ = (mandatory_label, audit_descriptor);
            }
            NativeMetadata::Macos {
                mode,
                flags,
                created_nanoseconds,
                modified_nanoseconds,
                ..
            } => {
                if *mode & !0o7777 != 0
                    || *flags != 0
                    || !(0..1_000_000_000).contains(created_nanoseconds)
                    || !(0..1_000_000_000).contains(modified_nanoseconds)
                {
                    return Err(invalid("Unsupported macOS flags or essential metadata."));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn digest(&self) -> Result<String> {
        self.validate()?;
        // Object identity is checked separately; a new output must be a
        // different object while reproducing the same essential metadata.
        let bytes = serde_json::to_vec(&(self.version, &self.policy, &self.metadata))
            .map_err(|_| invalid("Could not encode essential metadata."))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
    /// Ordinary owner/permissions/time metadata is distinct from certified
    /// replacement evidence. Missing Windows audit-security read rights remain
    /// unknown; no privilege is enabled merely to make this return true.
    #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
    pub(crate) fn certified_replacement(&self) -> Result<bool> {
        self.validate()?;
        Ok(match &self.metadata {
            NativeMetadata::Windows {
                audit_descriptor, ..
            } => audit_descriptor.is_some(),
            NativeMetadata::Macos { .. } => true,
        })
    }
    pub(crate) fn description(&self) -> &'static str {
        match &self.metadata {
            NativeMetadata::Windows { .. } => {
                "Metadata policy v1 records owner/group, ordinary allow/deny DACL, mandatory integrity label, attributes and creation/modification times. Named streams, extended attributes and advanced ACL entries are unsupported. Audit-security fidelity remains unknown unless the current native read rights prove the supported no-audit-content policy; no privileges are enabled."
            }
            NativeMetadata::Macos { .. } => {
                "Metadata policy v1 records UID/GID, POSIX permissions, creation/modification times and proven absence of extended ACL entries/flags, xattrs, resource forks and file flags. Unsupported native metadata blocks strict replacement."
            }
        }
    }
    pub(crate) fn compare(&self, file: &File) -> Result<()> {
        self.validate()?;
        let actual = Self::inspect(file)?;
        if actual.metadata != self.metadata || actual.policy != self.policy {
            let detail = match (&self.metadata, &actual.metadata) {
                (
                    NativeMetadata::Windows {
                        attributes: a,
                        created: b,
                        modified: c,
                        security_descriptor: d,
                        mandatory_label: e,
                        audit_descriptor: f,
                    },
                    NativeMetadata::Windows {
                        attributes: g,
                        created: h,
                        modified: i,
                        security_descriptor: j,
                        mandatory_label: k,
                        audit_descriptor: l,
                    },
                ) => {
                    #[cfg(windows)]
                    let security = format!(
                        "{}; {}",
                        windows::difference(d, j),
                        windows::label_difference(e, k)
                    );
                    #[cfg(not(windows))]
                    let security = String::from("foreign Windows security");
                    format!(
                        "Metadata field mismatch: attributes={},creation_time={},modification_time={},security={},mandatory_label={},audit={}; {}",
                        a != g,
                        b != h,
                        c != i,
                        d != j,
                        e != k,
                        f != l,
                        security
                    )
                }
                (
                    NativeMetadata::Macos {
                        uid: a,
                        gid: b,
                        mode: c,
                        flags: d,
                        created_seconds: e,
                        created_nanoseconds: f,
                        modified_seconds: g,
                        modified_nanoseconds: h,
                    },
                    NativeMetadata::Macos {
                        uid: i,
                        gid: j,
                        mode: k,
                        flags: l,
                        created_seconds: m,
                        created_nanoseconds: n,
                        modified_seconds: o,
                        modified_nanoseconds: p,
                    },
                ) => format!(
                    "Metadata field mismatch: owner={},group={},permissions={},flags={},creation_time={},modification_time={}",
                    a != i,
                    b != j,
                    c != k,
                    d != l,
                    (e, f) != (m, n),
                    (g, h) != (o, p)
                ),
                _ => "Metadata platform/policy mismatch.".into(),
            };
            return Err(changed().caused_by(detail));
        }
        Ok(())
    }
    /// Only central adapters holding a journal-bound newly created output may
    /// call this. It cannot target the captured original object, even after a
    /// rename. No paths, renderer data or original handles are execution inputs.
    pub(crate) fn restore_new(&self, file: &File) -> Result<()> {
        self.validate()?;
        let target = filesystem::identity(file)?;
        if target.links != 1
            || !file.metadata()?.is_file()
            || (target.volume == self.origin.volume && target.file == self.origin.file)
        {
            return Err(invalid(
                "Essential metadata can only be restored into a different newly created owned file.",
            ));
        }
        restore_native(file, &self.metadata)?;
        if filesystem::identity(file)? != target {
            return Err(changed());
        }
        self.compare(file)
    }
}

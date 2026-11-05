//! OS-only activation selector. Files/catalog rows never advance this authority.
use super::*;

const MAX_AUTHORITY: usize = 1536;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selector {
    pub transaction_id: Option<String>,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pending {
    pub generation: u64,
    pub expected_generation: u64,
    pub expected_active: Selector,
    pub selector: Selector,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Authority {
    version: u32,
    pub generation: u64,
    pub initializing: bool,
    pub active: Selector,
    pub pending: Option<Pending>,
}
fn rejected() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "Catalog activation authority is unavailable, stale or uncertain. Preserve every catalog; no empty catalog or rollback is selected.",
    )
}
fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn key(scope: &str) -> Result<String> {
    if !hash(scope) {
        return Err(rejected());
    }
    Ok(format!("{SERVICE}/catalog-activation/{scope}"))
}
impl Selector {
    fn validate(&self) -> Result<()> {
        if !hash(&self.digest)
            || self
                .transaction_id
                .as_ref()
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err(rejected());
        }
        Ok(())
    }
}
impl Authority {
    fn validate(&self) -> Result<()> {
        self.active.validate()?;
        if self.version != 1
            || (self.initializing
                && (self.generation != 0
                    || self.active.transaction_id.is_some()
                    || self.pending.is_some()))
        {
            return Err(rejected());
        }
        if let Some(pending) = &self.pending {
            pending.selector.validate()?;
            if pending.expected_active != self.active
                || pending.expected_generation != self.generation
                || Some(pending.generation) != self.generation.checked_add(1)
                || pending.selector.transaction_id.is_none()
                || pending.selector == self.active
            {
                return Err(rejected());
            }
        }
        Ok(())
    }
}
pub(crate) fn read(scope: &str) -> Result<Option<Authority>> {
    let Some(bytes) = platform::get(&key(scope)?)? else {
        return Ok(None);
    };
    if bytes.len() > MAX_AUTHORITY || !bytes.starts_with(b"SHCA1") {
        return Err(rejected());
    }
    let authority: Authority = serde_json::from_slice(&bytes[5..]).map_err(|_| rejected())?;
    authority.validate()?;
    Ok(Some(authority))
}
/// Exact-scope foreground authorization, never credential reenrollment.
pub(crate) fn authorize(scope: &str) -> Result<()> {
    let key = key(scope)?;
    #[cfg(target_os = "macos")]
    {
        platform::authorize(&key)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = key;
        Err(rejected())
    }
}
fn write(scope: &str, value: &Authority) -> Result<()> {
    value.validate()?;
    let mut bytes = Zeroizing::new(b"SHCA1".to_vec());
    bytes.extend(serde_json::to_vec(value).map_err(|_| rejected())?);
    if bytes.len() > MAX_AUTHORITY {
        return Err(rejected());
    }
    platform::set(&key(scope)?, &bytes)?;
    #[cfg(test)]
    if tests::fail_after_write(scope) {
        return Err(rejected());
    }
    if read(scope)?.as_ref() != Some(value) {
        return Err(rejected());
    }
    Ok(())
}
// The caller must hold the native appdata lifetime lock throughout every read /
// compare / write. This module is crate-private and has no IPC representation.
pub(crate) fn initialize(scope: &str, active: Selector, initializing: bool) -> Result<Authority> {
    if read(scope)?.is_some() {
        return Err(rejected());
    }
    let value = Authority {
        version: 1,
        generation: 0,
        initializing,
        active,
        pending: None,
    };
    write(scope, &value)?;
    Ok(value)
}
pub(crate) fn stage(scope: &str, expected: &Authority, selector: Selector) -> Result<Authority> {
    if read(scope)?.as_ref() != Some(expected)
        || expected.pending.is_some()
        || expected.initializing
    {
        return Err(rejected());
    }
    let mut value = expected.clone();
    value.pending = Some(Pending {
        generation: expected.generation.checked_add(1).ok_or_else(rejected)?,
        expected_generation: expected.generation,
        expected_active: expected.active.clone(),
        selector,
    });
    write(scope, &value)?;
    Ok(value)
}
pub(crate) fn promote(scope: &str, expected: &Authority) -> Result<Authority> {
    if read(scope)?.as_ref() != Some(expected) {
        return Err(rejected());
    }
    let pending = expected.pending.as_ref().ok_or_else(rejected)?;
    let value = Authority {
        version: 1,
        generation: pending.generation,
        initializing: false,
        active: pending.selector.clone(),
        pending: None,
    };
    write(scope, &value)?;
    Ok(value)
}
pub(crate) fn confirm_initialized(scope: &str, expected: &Authority) -> Result<Authority> {
    if read(scope)?.as_ref() != Some(expected) || !expected.initializing {
        return Err(rejected());
    }
    let mut value = expected.clone();
    value.initializing = false;
    value.generation = value.generation.checked_add(1).ok_or_else(rejected)?;
    write(scope, &value)?;
    Ok(value)
}
#[cfg(test)]
pub(crate) fn delete_owned_fixture(scope: &str) -> Result<()> {
    platform::delete(&key(scope)?)
}
#[cfg(feature = "fixture-probes")]
pub(crate) fn delete_generated_session(scope: &str) -> Result<()> {
    platform::delete(&key(scope)?)
}

#[cfg(test)]
mod tests {
    #[cfg(any(windows, target_os = "macos"))]
    use super::*;
    use std::{
        collections::BTreeSet,
        sync::{Mutex, OnceLock},
    };
    fn faults() -> &'static Mutex<BTreeSet<String>> {
        static FAULTS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
        FAULTS.get_or_init(Default::default)
    }
    pub(super) fn fail_after_write(scope: &str) -> bool {
        faults().lock().unwrap().remove(scope)
    }
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn native_activation_authority_uncertain_write_and_stale_generations_cannot_rollback() {
        struct Owned(String);
        impl Drop for Owned {
            fn drop(&mut self) {
                let result = delete_owned_fixture(&self.0);
                if !std::thread::panicking() {
                    result.unwrap();
                }
            }
        }
        let owned = Owned(
            blake3::hash(uuid::Uuid::new_v4().as_bytes())
                .to_hex()
                .to_string(),
        );
        let initial = initialize(
            &owned.0,
            Selector {
                transaction_id: None,
                digest: "a".repeat(64),
            },
            false,
        )
        .unwrap();
        let chosen = Selector {
            transaction_id: Some(uuid::Uuid::new_v4().to_string()),
            digest: "b".repeat(64),
        };
        faults().lock().unwrap().insert(owned.0.clone());
        assert!(
            stage(&owned.0, &initial, chosen.clone()).is_err(),
            "OS commit followed by an uncertain acknowledgment is not success"
        );
        let pending = read(&owned.0).unwrap().unwrap();
        assert_eq!(pending.active, initial.active);
        assert_eq!(pending.pending.as_ref().unwrap().selector, chosen);
        assert!(stage(&owned.0, &initial, chosen.clone()).is_err());
        let active = promote(&owned.0, &pending).unwrap();
        assert_eq!(active.generation, 1);
        assert_eq!(active.active, chosen);
        assert!(promote(&owned.0, &pending).is_err());
        assert!(stage(&owned.0, &initial, initial.active.clone()).is_err());
        assert_eq!(read(&owned.0).unwrap().unwrap(), active);
    }
}

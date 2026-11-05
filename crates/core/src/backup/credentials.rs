//! Catalog/purpose-bound OS credentials. Secret bytes have no IPC representation.
use crate::domain::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeroize::Zeroizing;
pub(crate) mod catalog_restore;

const SERVICE: &str = "stillharbour.backup.v1";
const MAX_SECRET: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CredentialPurpose {
    Repository,
    Transport,
}
impl CredentialPurpose {
    fn name(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::Transport => "transport",
        }
    }
}
pub struct Secret(Zeroizing<Vec<u8>>);
impl Secret {
    #[cfg(test)]
    pub(crate) fn fixture(secret: &str) -> Self {
        Self(Zeroizing::new(secret.as_bytes().to_vec()))
    }
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.0).map_err(|_| unavailable())
    }
}
pub struct CredentialStore;
fn unavailable() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "The OS credential is locked, unavailable or belongs to another catalog/purpose. Use native credential setup again.",
    )
}
fn account(catalog_id: &str, purpose: CredentialPurpose, reference: &str) -> Result<String> {
    if uuid::Uuid::parse_str(catalog_id).is_err() || uuid::Uuid::parse_str(reference).is_err() {
        return Err(unavailable());
    }
    Ok(format!(
        "{SERVICE}/{catalog_id}/{}/{reference}",
        purpose.name()
    ))
}
fn binding(purpose: CredentialPurpose, context: Option<&str>) -> Result<String> {
    match context {
        Some(value) if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Ok(value.into())
        }
        None if purpose == CredentialPurpose::Repository => Ok("0".repeat(64)),
        _ => Err(AppError::new(
            ErrorCode::InvalidInput,
            "Transport credentials require the reviewed endpoint context.",
        )),
    }
}
fn encoded(
    purpose: CredentialPurpose,
    context: Option<&str>,
    secret: &str,
) -> Result<Zeroizing<Vec<u8>>> {
    if secret.is_empty()
        || secret.len() > MAX_SECRET
        || secret.chars().any(|c| matches!(c, '\0' | '\r' | '\n'))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "A credential must be one nonempty line of at most 2048 UTF-8 bytes.",
        ));
    }
    let mut value = Zeroizing::new(b"SHC1".to_vec());
    value.extend_from_slice(binding(purpose, context)?.as_bytes());
    value.extend_from_slice(secret.as_bytes());
    Ok(value)
}
fn decoded(
    purpose: CredentialPurpose,
    context: Option<&str>,
    value: Zeroizing<Vec<u8>>,
) -> Result<Secret> {
    if value.len() <= 68
        || value.len() > MAX_SECRET + 68
        || &value[..4] != b"SHC1"
        || &value[4..68] != binding(purpose, context)?.as_bytes()
    {
        return Err(unavailable());
    }
    let secret = std::str::from_utf8(&value[68..]).map_err(|_| unavailable())?;
    if secret.chars().any(|c| matches!(c, '\0' | '\r' | '\n')) {
        return Err(unavailable());
    }
    Ok(Secret(Zeroizing::new(value[68..].to_vec())))
}
impl CredentialStore {
    pub fn available() -> bool {
        cfg!(any(windows, target_os = "macos"))
    }
    pub fn store(catalog_id: &str, purpose: CredentialPurpose, secret: &str) -> Result<String> {
        Self::store_bound(catalog_id, purpose, None, secret)
    }
    pub fn store_bound(
        catalog_id: &str,
        purpose: CredentialPurpose,
        context: Option<&str>,
        secret: &str,
    ) -> Result<String> {
        let reference = uuid::Uuid::new_v4().to_string();
        let key = account(catalog_id, purpose, &reference)?;
        platform::set(&key, &encoded(purpose, context, secret)?)?;
        Ok(reference)
    }
    pub fn get(catalog_id: &str, purpose: CredentialPurpose, reference: &str) -> Result<Secret> {
        Self::get_bound(catalog_id, purpose, reference, None)
    }
    pub fn get_bound(
        catalog_id: &str,
        purpose: CredentialPurpose,
        reference: &str,
        context: Option<&str>,
    ) -> Result<Secret> {
        let key = account(catalog_id, purpose, reference)?;
        decoded(
            purpose,
            context,
            platform::get(&key)?.ok_or_else(unavailable)?,
        )
    }
    /// Native cleanup only. No bulk deletion or credential enumeration exists.
    pub fn delete_owned(
        catalog_id: &str,
        purpose: CredentialPurpose,
        reference: &str,
    ) -> Result<()> {
        platform::delete(&account(catalog_id, purpose, reference)?)
    }
    /// Generated native-fixture assertion only. A locked/unavailable store is
    /// an error, never evidence that an exact scoped credential is absent.
    #[cfg(feature = "fixture-probes")]
    pub fn fixture_owned_absent(
        catalog_id: &str,
        purpose: CredentialPurpose,
        reference: &str,
    ) -> Result<bool> {
        Ok(platform::get(&account(catalog_id, purpose, reference)?)?.is_none())
    }
}
// Native recurring consent and its timing authority are separate from the catalog.
mod schedules;
pub(crate) use schedules::{ScheduleAuthority, ScheduleScope, schedule_transaction};
#[cfg(test)]
pub(crate) fn schedule_revoke(catalog: &str, instance: &str, profile: &str) -> Result<()> {
    schedule_transaction()?.revoke(catalog, instance, profile)
}

#[cfg(windows)]
mod platform {
    use super::*;
    use windows_sys::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    };
    use zeroize::Zeroize;
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }
    pub(super) fn set(key: &str, secret: &[u8]) -> Result<()> {
        let mut key = wide(key);
        let mut user = wide("Stillharbour backup");
        let mut bytes = Zeroizing::new(secret.to_vec());
        let record = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: key.as_mut_ptr(),
            UserName: user.as_mut_ptr(),
            CredentialBlobSize: bytes.len() as u32,
            CredentialBlob: bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        // SAFETY: NUL-terminated UTF-16 identifiers and live bounded secret buffer.
        if unsafe { CredWriteW(&record, 0) } == 0 {
            return Err(unavailable());
        }
        Ok(())
    }
    struct ReadRecord(*mut CREDENTIALW);
    impl Drop for ReadRecord {
        fn drop(&mut self) {
            // SAFETY: successful CredReadW allocates this record for CredFree.
            unsafe {
                let record = &mut *self.0;
                if !record.CredentialBlob.is_null() && record.CredentialBlobSize <= 2560 {
                    std::slice::from_raw_parts_mut(
                        record.CredentialBlob,
                        record.CredentialBlobSize as usize,
                    )
                    .zeroize();
                }
                CredFree(self.0.cast());
            }
        }
    }
    pub(super) fn get(key: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let key = wide(key);
        let mut raw = std::ptr::null_mut();
        // SAFETY: valid identifier and pointer to the output allocation.
        if unsafe { CredReadW(key.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) } == 0 {
            return if unsafe { windows_sys::Win32::Foundation::GetLastError() } == 1168 {
                Ok(None)
            } else {
                Err(unavailable())
            };
        }
        if raw.is_null() {
            return Err(unavailable());
        }
        let record = ReadRecord(raw);
        // SAFETY: allocation remains held by ReadRecord; validate bounds before slice.
        let value = unsafe { &*record.0 };
        if value.CredentialBlob.is_null() || value.CredentialBlobSize as usize > MAX_SECRET + 68 {
            return Err(unavailable());
        }
        // SAFETY: SDK guarantees a blob of CredentialBlobSize bytes in the allocation.
        Ok(Some(Zeroizing::new(
            unsafe {
                std::slice::from_raw_parts(value.CredentialBlob, value.CredentialBlobSize as usize)
            }
            .to_vec(),
        )))
    }
    pub(super) fn delete(key: &str) -> Result<()> {
        let key = wide(key);
        // SAFETY: exact catalog/purpose/UUID target only, no wildcard API.
        if unsafe { CredDeleteW(key.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            // Another exact-scope cleanup may have removed it after a presence
            // check; genuinely absent is still a successful revocation.
            if unsafe { windows_sys::Win32::Foundation::GetLastError() } != 1168 {
                return Err(unavailable());
            }
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        data::CFData,
        dictionary::CFDictionary,
        string::{CFString, CFStringRef},
    };
    use security_framework_sys::{
        item::*,
        keychain::SecKeychainSetUserInteractionAllowed,
        keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate},
    };
    // SecItem's query-level UI policy does not cover every file-keychain
    // authentication path. Serialize the process-wide legacy policy too. Only
    // the explicit native startup action may temporarily enable interaction.
    static INTERACTION: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct Interaction {
        _guard: std::sync::MutexGuard<'static, ()>,
    }
    impl Interaction {
        fn enter(allowed: bool) -> Result<Self> {
            let guard = INTERACTION.lock().map_err(|_| unavailable())?;
            // SAFETY: changes only this process's Keychain UI policy; no item,
            // ACL, login-keychain lock or password is changed.
            let status = unsafe { SecKeychainSetUserInteractionAllowed(u8::from(allowed)) };
            if status != 0 {
                return Err(keychain_error(status));
            }
            Ok(Self { _guard: guard })
        }
    }
    impl Drop for Interaction {
        fn drop(&mut self) {
            // SAFETY: the mutex remains held until after this call. Every
            // subsequent operation also sets deny before accessing an item.
            unsafe { SecKeychainSetUserInteractionAllowed(0) };
        }
    }
    fn keychain_error(status: i32) -> AppError {
        let message = match status {
            -25308 | -25315 | -25293 => {
                return AppError::new(
                    ErrorCode::CredentialAccessRequired,
                    "macOS Keychain needs permission for this build of Stillharbour. Unlock your login keychain if it is locked, then authorize access. Rebuilt ad-hoc development apps may need permission again; a consistently signed build retains its signing identity.",
                );
            }
            -25291 => {
                "macOS Keychain is unavailable. Sign in to your Mac and unlock your login keychain, then reopen Stillharbour."
            }
            -128 => {
                "macOS Keychain authorization was canceled. Reopen Stillharbour to try again; existing credentials and catalogs are unchanged."
            }
            _ => {
                "macOS Keychain could not complete the request. Existing credentials and catalogs have been retained."
            }
        };
        AppError::new(
            ErrorCode::Unsupported,
            format!("{message} (Keychain status {status}.)"),
        )
    }
    // The pinned sys crate exposes UI Skip but not UI Fail. Skip deliberately
    // hides protected items as absent, which cannot certify consent revocation.
    // This SDK constant is available on all supported macOS versions.
    unsafe extern "C" {
        #[link_name = "kSecUseAuthenticationUIFail"]
        static AUTHENTICATION_UI_FAIL: CFStringRef;
    }
    fn query(key: &str) -> Vec<(CFString, CFType)> {
        // SAFETY: Security.framework exports immortal CFString constants;
        // wrap-under-get retains them for the dictionary's lifetime.
        unsafe {
            vec![
                (
                    CFString::wrap_under_get_rule(kSecClass),
                    CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrService),
                    CFString::new(SERVICE).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccount),
                    CFString::new(key).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrSynchronizable),
                    CFBoolean::false_value().as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecUseAuthenticationUI),
                    CFString::wrap_under_get_rule(AUTHENTICATION_UI_FAIL).as_CFType(),
                ),
            ]
        }
    }
    pub(super) fn set(key: &str, secret: &[u8]) -> Result<()> {
        if secret.len() > MAX_SECRET + 68 {
            return Err(unavailable());
        }
        let _interaction = Interaction::enter(false)?;
        let mut pairs = query(key);
        // SAFETY: immortal Security constant, retained by the dictionary.
        let data_key = unsafe { CFString::wrap_under_get_rule(kSecValueData) };
        let data = CFData::from_buffer(secret);
        pairs.push((data_key.clone(), data.as_CFType()));
        let add = CFDictionary::from_CFType_pairs(&pairs);
        // SAFETY: dictionaries hold valid retained CF objects; no output requested.
        let status = unsafe { SecItemAdd(add.as_concrete_TypeRef(), std::ptr::null_mut()) };
        if status == 0 {
            return Ok(());
        }
        if status != -25299 {
            return Err(keychain_error(status));
        }
        let matching = CFDictionary::from_CFType_pairs(&query(key));
        let update = CFDictionary::from_CFType_pairs(&[(data_key, data.as_CFType())]);
        // SAFETY: exact service/account query and retained bounded value; UI Fail
        // remains in the query, including for background schedule marker writes.
        let status =
            unsafe { SecItemUpdate(matching.as_concrete_TypeRef(), update.as_concrete_TypeRef()) };
        if status != 0 {
            return Err(keychain_error(status));
        }
        Ok(())
    }
    pub(super) fn get(key: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let _interaction = Interaction::enter(false)?;
        read(key, false)
    }
    /// Native catalog startup only, after the owner's explicit Authorize action.
    /// One exact existing item; never an enumeration, write or ACL replacement.
    pub(super) fn authorize(key: &str) -> Result<()> {
        let _interaction = Interaction::enter(true)?;
        let _value = read(key, true)?;
        Ok(())
    }
    fn read(key: &str, authorize: bool) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let mut pairs = query(key);
        if authorize {
            // SAFETY: immortal SDK constant. Omit UI Fail only for this one
            // foreground call; ordinary reads keep both denial mechanisms.
            let ui = unsafe { CFString::wrap_under_get_rule(kSecUseAuthenticationUI) };
            pairs.retain(|(key, _)| key != &ui);
        }
        // SAFETY: immortal Security constant retained by the dictionary.
        pairs.push((
            unsafe { CFString::wrap_under_get_rule(kSecReturnData) },
            CFBoolean::true_value().as_CFType(),
        ));
        let query = CFDictionary::from_CFType_pairs(&pairs);
        let mut raw = std::ptr::null();
        // SAFETY: valid retained query and output pointer. UI Fail distinguishes
        // locked/auth-required items from genuinely absent items without a popup.
        let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut raw) };
        if status == -25300 {
            return Ok(None);
        }
        if status != 0 {
            return Err(keychain_error(status));
        }
        if raw.is_null() {
            return Err(unavailable());
        }
        // SAFETY: successful Copy transfers one ownership reference.
        let value = unsafe { CFType::wrap_under_create_rule(raw) };
        let data = value.downcast::<CFData>().ok_or_else(unavailable)?;
        if data.len() > (MAX_SECRET + 68) as isize {
            return Err(unavailable());
        }
        Ok(Some(Zeroizing::new(data.bytes().to_vec())))
    }
    pub(super) fn delete(key: &str) -> Result<()> {
        let _interaction = Interaction::enter(false)?;
        let query = CFDictionary::from_CFType_pairs(&query(key));
        // SAFETY: exact service/account only, no wildcard or interaction fallback.
        match unsafe { SecItemDelete(query.as_concrete_TypeRef()) } {
            0 | -25300 => Ok(()),
            status => Err(keychain_error(status)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn authentication_failures_are_not_missing_items_or_reenrollment_requests() {
            for status in [-25308, -25315, -25293] {
                let error = keychain_error(status);
                assert_eq!(error.code, ErrorCode::CredentialAccessRequired);
                assert!(error.message.contains("permission"));
                assert!(!error.message.contains("setup again"));
            }
            for status in [-25291, -128, -34018, -25294] {
                let error = keychain_error(status);
                assert_eq!(error.code, ErrorCode::Unsupported);
                assert!(error.message.contains(&status.to_string()));
            }
        }
        #[test]
        fn file_keychain_access_disables_implicit_password_dialogs() {
            let key = format!("generated-no-prompt-probe/{}", uuid::Uuid::new_v4());
            assert!(get(&key).unwrap().is_none());
            let mut allowed = 1;
            // SAFETY: valid Boolean output; reads only this process's UI policy.
            assert_eq!(
                unsafe {
                    security_framework_sys::keychain::SecKeychainGetUserInteractionAllowed(
                        &mut allowed,
                    )
                },
                0
            );
            assert_eq!(
                allowed, 0,
                "file-based Keychain must not prompt during catalog startup or polling"
            );
        }
        #[test]
        fn every_keychain_operation_uses_fail_instead_of_hiding_authenticated_items() {
            let pairs = query("generated-nonpersistent-query-test");
            // SAFETY: assertions read immortal SDK constants; no OS item is read
            // or changed and no authentication interaction is requested.
            unsafe {
                let ui = CFString::wrap_under_get_rule(kSecUseAuthenticationUI);
                let fail = CFString::wrap_under_get_rule(AUTHENTICATION_UI_FAIL).as_CFType();
                let sync = CFString::wrap_under_get_rule(kSecAttrSynchronizable);
                assert!(
                    pairs
                        .iter()
                        .any(|(key, value)| key == &ui && value == &fail)
                );
                assert!(
                    pairs.iter().any(|(key, value)| key == &sync
                        && value == &CFBoolean::false_value().as_CFType())
                );
            }
        }
    }
}
#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use super::*;
    pub(super) fn set(_: &str, _: &[u8]) -> Result<()> {
        Err(unavailable())
    }
    pub(super) fn get(_: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        Err(unavailable())
    }
    pub(super) fn delete(_: &str) -> Result<()> {
        Err(unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_are_bound_to_catalog_and_purpose_without_secret_serialization() {
        let a = uuid::Uuid::new_v4().to_string();
        let b = uuid::Uuid::new_v4().to_string();
        let r = uuid::Uuid::new_v4().to_string();
        assert_ne!(
            account(&a, CredentialPurpose::Repository, &r).unwrap(),
            account(&b, CredentialPurpose::Repository, &r).unwrap()
        );
        assert_ne!(
            account(&a, CredentialPurpose::Repository, &r).unwrap(),
            account(&a, CredentialPurpose::Transport, &r).unwrap()
        );
        assert!(account("foreign", CredentialPurpose::Repository, &r).is_err());
        assert!(CredentialStore::store(&a, CredentialPurpose::Repository, "x\ny").is_err());
    }
    #[test]
    fn maximum_secret_survives_envelope_and_endpoint_binding_is_exact() {
        let secret = "x".repeat(MAX_SECRET);
        let context = "a".repeat(64);
        let payload = encoded(CredentialPurpose::Transport, Some(&context), &secret).unwrap();
        assert_eq!(payload.len(), MAX_SECRET + 68);
        assert_eq!(
            decoded(CredentialPurpose::Transport, Some(&context), payload)
                .unwrap()
                .as_str()
                .unwrap(),
            secret
        );
        let payload = encoded(CredentialPurpose::Transport, Some(&context), &secret).unwrap();
        assert!(decoded(CredentialPurpose::Transport, Some(&"b".repeat(64)), payload).is_err());
        assert!(
            encoded(
                CredentialPurpose::Repository,
                None,
                &"x".repeat(MAX_SECRET + 1)
            )
            .is_err()
        );
    }
}

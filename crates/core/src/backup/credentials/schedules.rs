//! OS-backed recurring consent and a monotonic native timing high-water mark.
//! The process-wide transaction covers enable/disable/read/claim/start admission;
//! the native catalog lifetime lock excludes a second catalog process.
use super::*;
use chrono::{DateTime, Utc};
use std::sync::{Mutex, MutexGuard};
const MAX_AUTHORITY: usize = 1024;
static SCHEDULE_TRANSACTION: Mutex<()> = Mutex::new(());
pub(crate) struct ScheduleScope<'a> {
    pub catalog_id: &'a str,
    pub instance: &'a str,
    pub profile_id: &'a str,
    pub policy_digest: &'a str,
}
fn schedule_unavailable() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "Schedule authorization or timing is unavailable. Create a fresh exact-scope review and enable the schedule again.",
    )
}
fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn key(catalog: &str, instance: &str, profile: &str) -> Result<String> {
    if uuid::Uuid::parse_str(catalog).is_err()
        || uuid::Uuid::parse_str(profile).is_err()
        || !hash(instance)
    {
        return Err(schedule_unavailable());
    }
    Ok(format!("{SERVICE}/schedule/{catalog}/{instance}/{profile}"))
}
impl ScheduleScope<'_> {
    fn key(&self) -> Result<String> {
        if !hash(self.policy_digest) {
            return Err(schedule_unavailable());
        }
        key(self.catalog_id, self.instance, self.profile_id)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScheduleAuthority {
    version: u32,
    pub policy_digest: String,
    pub generation: String,
    pub next_admissible_utc: String,
    pub last_attempt_utc: Option<String>,
}
impl ScheduleAuthority {
    pub(crate) fn deadline(&self) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.next_admissible_utc)
            .map(|v| v.with_timezone(&Utc))
            .map_err(|_| schedule_unavailable())
    }
    fn validate(&self) -> Result<()> {
        if self.version != 2
            || !hash(&self.policy_digest)
            || uuid::Uuid::parse_str(&self.generation).is_err()
            || self.next_admissible_utc.len() > 64
        {
            return Err(schedule_unavailable());
        }
        let deadline = self.deadline()?;
        if let Some(last) = &self.last_attempt_utc
            && (last.len() > 64
                || DateTime::parse_from_rfc3339(last)
                    .map_err(|_| schedule_unavailable())?
                    .with_timezone(&Utc)
                    >= deadline)
        {
            return Err(schedule_unavailable());
        }
        Ok(())
    }
}
pub(crate) struct ScheduleTransaction {
    _guard: MutexGuard<'static, ()>,
    #[cfg(test)]
    fail_after_commit: bool,
}
pub(crate) fn schedule_transaction() -> Result<ScheduleTransaction> {
    Ok(ScheduleTransaction {
        _guard: SCHEDULE_TRANSACTION
            .lock()
            .map_err(|_| schedule_unavailable())?,
        #[cfg(test)]
        fail_after_commit: false,
    })
}
impl ScheduleTransaction {
    #[cfg(test)]
    pub(crate) fn fail_next_write_after_commit(&mut self) {
        self.fail_after_commit = true;
    }
    #[cfg(test)]
    pub(crate) fn write_legacy_for_test(&mut self, scope: &ScheduleScope<'_>) -> Result<()> {
        platform::set(
            &scope.key()?,
            format!("SHS1{}", scope.policy_digest).as_bytes(),
        )
    }

    pub(crate) fn read(&self, scope: &ScheduleScope<'_>) -> Result<Option<ScheduleAuthority>> {
        let key = scope.key()?;
        let Some(bytes) = platform::get(&key)? else {
            return Ok(None);
        };
        // A digest-only permit cannot create timing authority on upgrade.
        if bytes.starts_with(b"SHS1") {
            return Ok(None);
        }
        if bytes.len() > MAX_AUTHORITY || !bytes.starts_with(b"SHS2") {
            return Err(schedule_unavailable());
        }
        let authority: ScheduleAuthority =
            serde_json::from_slice(&bytes[4..]).map_err(|_| schedule_unavailable())?;
        authority.validate()?;
        if authority.policy_digest != scope.policy_digest {
            return Ok(None);
        }
        Ok(Some(authority))
    }
    fn write(&mut self, scope: &ScheduleScope<'_>, authority: &ScheduleAuthority) -> Result<()> {
        authority.validate()?;
        if authority.policy_digest != scope.policy_digest {
            return Err(schedule_unavailable());
        }
        let mut bytes = b"SHS2".to_vec();
        bytes.extend(serde_json::to_vec(authority).map_err(|_| schedule_unavailable())?);
        if bytes.len() > MAX_AUTHORITY {
            return Err(schedule_unavailable());
        }
        platform::set(&scope.key()?, &bytes)?;
        #[cfg(test)]
        if std::mem::take(&mut self.fail_after_commit) {
            return Err(schedule_unavailable());
        }

        // Uncertain write/readback never earns permission to spawn. Do not roll
        // the native high-water mark backward if this verification fails.
        if self.read(scope)?.as_ref() != Some(authority) {
            return Err(schedule_unavailable());
        }
        Ok(())
    }
    pub(crate) fn authorize(
        &mut self,
        scope: &ScheduleScope<'_>,
        deadline: DateTime<Utc>,
    ) -> Result<ScheduleAuthority> {
        let authority = ScheduleAuthority {
            version: 2,
            policy_digest: scope.policy_digest.into(),
            generation: uuid::Uuid::new_v4().to_string(),
            next_admissible_utc: deadline.to_rfc3339(),
            last_attempt_utc: None,
        };
        self.write(scope, &authority)?;
        Ok(authority)
    }
    pub(crate) fn claim(
        &mut self,
        scope: &ScheduleScope<'_>,
        expected: &ScheduleAuthority,
        now: DateTime<Utc>,
        interval_minutes: u32,
        attempt: bool,
    ) -> Result<Option<ScheduleAuthority>> {
        if !(15..=10080).contains(&interval_minutes) {
            return Err(schedule_unavailable());
        }
        let current = self.read(scope)?.ok_or_else(schedule_unavailable)?;
        if &current != expected {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Schedule authority changed before admission; no run was started.",
            ));
        }
        if now < current.deadline()? {
            return Ok(None);
        }
        let deadline = now
            .checked_add_signed(chrono::Duration::minutes(i64::from(interval_minutes)))
            .ok_or_else(schedule_unavailable)?;
        let advanced = ScheduleAuthority {
            version: 2,
            policy_digest: current.policy_digest,
            generation: uuid::Uuid::new_v4().to_string(),
            next_admissible_utc: deadline.to_rfc3339(),
            last_attempt_utc: if attempt {
                Some(now.to_rfc3339())
            } else {
                current.last_attempt_utc
            },
        };
        self.write(scope, &advanced)?;
        Ok(Some(advanced))
    }
    pub(crate) fn revoke(&mut self, catalog: &str, instance: &str, profile: &str) -> Result<()> {
        platform::delete(&key(catalog, instance, profile)?)
    }
}

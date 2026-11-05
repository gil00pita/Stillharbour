//! One explicit foreground authorization attempt; never an automatic prompt loop.
use stillharbour_core::domain::{ErrorCode, Result};

pub fn select<T>(
    mut selection: impl FnMut() -> Result<T>,
    confirm: impl FnOnce(&str) -> bool,
    authorize: impl FnOnce() -> Result<()>,
) -> Result<T> {
    match selection() {
        Err(error) if error.code == ErrorCode::CredentialAccessRequired => {
            if !confirm(&error.message) {
                return Err(error);
            }
            authorize()?;
            selection()
        }
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use stillharbour_core::domain::AppError;
    fn locked() -> AppError {
        AppError::new(
            ErrorCode::CredentialAccessRequired,
            "Keychain permission required",
        )
    }
    #[test]
    fn successful_startup_never_requests_password_or_authorization() {
        assert_eq!(
            select(
                || Ok(7),
                |_| panic!("no prompt"),
                || panic!("no authorization")
            )
            .unwrap(),
            7
        );
    }
    #[test]
    fn explicit_authorization_retries_the_real_selection_once() {
        let authorized = Cell::new(false);
        let calls = Cell::new(0);
        assert_eq!(
            select(
                || {
                    calls.set(calls.get() + 1);
                    if authorized.get() {
                        Ok(7)
                    } else {
                        Err(locked())
                    }
                },
                |_| true,
                || {
                    authorized.set(true);
                    Ok(())
                },
            )
            .unwrap(),
            7
        );
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn denied_authorization_cannot_loop_or_create_a_fallback_catalog() {
        let calls = Cell::new(0);
        let result: Result<()> = select(
            || {
                calls.set(calls.get() + 1);
                Err(locked())
            },
            |_| true,
            || Ok(()),
        );
        assert_eq!(
            result.unwrap_err().code,
            ErrorCode::CredentialAccessRequired
        );
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn canceled_prompt_and_failed_authorization_do_not_retry() {
        for confirmed in [false, true] {
            let calls = Cell::new(0);
            let result: Result<()> = select(
                || {
                    calls.set(calls.get() + 1);
                    Err(locked())
                },
                |_| confirmed,
                || Err(locked()),
            );
            assert!(result.is_err());
            assert_eq!(calls.get(), 1);
        }
    }
    #[test]
    fn catalog_errors_never_offer_keychain_authorization() {
        let result: Result<()> = select(
            || Err(AppError::new(ErrorCode::Conflict, "stale catalog selector")),
            |_| panic!("not a Keychain error"),
            || panic!("no authorization"),
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::Conflict);
    }
}

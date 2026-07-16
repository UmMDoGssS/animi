use crate::domain::{CommandError, SmokeResponse};

/// Verifies the serialized command boundary without touching durable state.
#[tauri::command(rename_all = "camelCase")]
pub fn smoke_check(simulate_error: Option<bool>) -> Result<SmokeResponse, CommandError> {
    if simulate_error.unwrap_or(false) {
        return Err(CommandError::new(
            "SMOKE_FAILURE",
            "Requested smoke failure",
            false,
        ));
    }

    Ok(SmokeResponse::ready())
}

#[cfg(test)]
mod tests {
    use super::smoke_check;

    #[test]
    fn returns_the_typed_success_contract() {
        let response = smoke_check(None).expect("the success path should return a response");

        assert_eq!(response.service, "animi-core");
        assert_eq!(response.message, "Local core is ready");
    }

    #[test]
    fn returns_the_structured_error_contract() {
        let error = smoke_check(Some(true)).expect_err("the requested error path should fail");

        assert_eq!(error.code, "SMOKE_FAILURE");
        assert!(!error.retryable);
    }
}

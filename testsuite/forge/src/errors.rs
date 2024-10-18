use anyhow::Error;
use thiserror::Error;

/// Represents the different types of errors that can occur during a Forge test
#[derive(Error, Debug, Clone)]
pub enum ForgeError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Infrastructure error: {0}")]
    InfraError(String),

    /// This is the default error type. Over time, errors should be categorized
    /// into more specific error types if possible.
    #[error("Unknown error: {0}")]
    UnknownError(String),
}

impl ForgeError {
    pub fn log_err(&self) {
        let error_type = match self {
            ForgeError::ValidationError(_) => "ValidationError",
            ForgeError::InfraError(_) => "InfraError",
            ForgeError::UnknownError(_) => "UnknownError",
        };

        aptos_logger::error!(
            target = "forge",
            event = "error_occurred",
            error_type = error_type,
            error_message = %self,
            "An error occurred during the Forge test"
        );
    }
}

impl From<anyhow::Error> for ForgeError {
    /// Converts an `anyhow::Error` to a `ForgeError`
    fn from(err: anyhow::Error) -> Self {
        ForgeError::UnknownError(err.to_string())
    }
}

/// Wrap an error in `ForgeError::UnknownError` if it's not already a
/// `ForgeError`
pub fn wrap_non_forge_error(err: Error, context: &str) -> ForgeError {
    if let Some(forge_err) = err.downcast_ref::<ForgeError>() {
        forge_err.clone()
    } else {
        ForgeError::UnknownError(format!("{}: {}", context, err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    fn raises_validation_error() -> Result<(), ForgeError> {
        Err(ForgeError::ValidationError(
            "Validation threshold exceeded".to_string(),
        ))
    }

    fn raises_infra_error() -> Result<(), ForgeError> {
        Err(ForgeError::InfraError("Infrastructure error".to_string()))
    }

    fn raises_unknown_error() -> Result<(), ForgeError> {
        Err(ForgeError::UnknownError("Unknown error".to_string()))
    }

    #[test]
    fn test_forge_error_log_err() {
        aptos_logger::Logger::init_for_testing();

        if let Err(e) = raises_validation_error() {
            e.log_err();
        }
        if let Err(e) = raises_infra_error() {
            e.log_err();
        }
        if let Err(e) = raises_unknown_error() {
            e.log_err();
        }
    }
}

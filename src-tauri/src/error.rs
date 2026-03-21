use serde::Serialize;

/// Result type alias for backend operations.
pub type Result<T> = std::result::Result<T, AppError>;

/// Structured backend error type exposed to commands and services.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    ConfigInvalid { field: String, reason: String },
    InvalidOperation { operation: String, reason: String },
    IoError { message: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigInvalid { field, reason } => {
                write!(f, "invalid config [{field}]: {reason}")
            }
            Self::InvalidOperation { operation, reason } => {
                write!(f, "invalid operation [{operation}]: {reason}")
            }
            Self::IoError { message } => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            message: err.to_string(),
        }
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        Self::ConfigInvalid {
            field: "toml".to_string(),
            reason: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_config_invalid() {
        let err = AppError::ConfigInvalid {
            field: "work_minutes".to_string(),
            reason: "must be > 0".to_string(),
        };
        let json = serde_json::to_value(&err).expect("error should serialize");
        assert_eq!(json["kind"], "ConfigInvalid");
        assert_eq!(json["detail"]["field"], "work_minutes");
    }

    #[test]
    fn serialize_io_error() {
        let err = AppError::IoError {
            message: "file not found".to_string(),
        };
        let json = serde_json::to_value(&err).expect("error should serialize");
        assert_eq!(json["kind"], "IoError");
    }

    #[test]
    fn serialize_invalid_operation() {
        let err = AppError::InvalidOperation {
            operation: "start_rest".to_string(),
            reason: "invalid in working".to_string(),
        };
        let json = serde_json::to_value(&err).expect("error should serialize");
        assert_eq!(json["kind"], "InvalidOperation");
        assert_eq!(json["detail"]["operation"], "start_rest");
    }

    #[test]
    fn display_format() {
        let err = AppError::InvalidOperation {
            operation: "timer event StartRest".to_string(),
            reason: "invalid in Working".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid operation [timer event StartRest]: invalid in Working"
        );
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let app_err = AppError::from(io_err);
        assert!(matches!(app_err, AppError::IoError { .. }));
    }
}

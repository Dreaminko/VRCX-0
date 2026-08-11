use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
enum AppErrorCode {
    Database,
    Io,
    Json,
    Custom,
}

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SqliteErrorCategory {
    Malformed,
    DiskFull,
    Locked,
    IoError,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct AppErrorPayload {
    code: AppErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sqlite_category: Option<SqliteErrorCategory>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {message}")]
    Database {
        message: String,
        sqlite_category: Option<SqliteErrorCategory>,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Custom(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload {
            code: self.code(),
            message: self.to_string(),
            sqlite_category: self.sqlite_category(),
        }
        .serialize(serializer)
    }
}

impl specta::Type for AppError {
    fn inline(
        type_map: &mut specta::TypeCollection,
        generics: specta::Generics,
    ) -> specta::DataType {
        AppErrorPayload::inline(type_map, generics)
    }

    fn reference(
        type_map: &mut specta::TypeCollection,
        generics: &[specta::DataType],
    ) -> specta::datatype::reference::Reference {
        AppErrorPayload::reference(type_map, generics)
    }
}

impl AppError {
    fn database(
        message: String,
        sqlite_category: Option<vrcx_0_persistence::SqliteErrorCategory>,
    ) -> Self {
        Self::Database {
            message,
            sqlite_category: sqlite_category.map(SqliteErrorCategory::from),
        }
    }

    fn code(&self) -> AppErrorCode {
        match self {
            Self::Database { .. } => AppErrorCode::Database,
            Self::Io(_) => AppErrorCode::Io,
            Self::Json(_) => AppErrorCode::Json,
            Self::Custom(_) => AppErrorCode::Custom,
        }
    }

    fn sqlite_category(&self) -> Option<SqliteErrorCategory> {
        match self {
            Self::Database {
                sqlite_category, ..
            } => *sqlite_category,
            _ => None,
        }
    }
}

impl From<vrcx_0_persistence::SqliteErrorCategory> for SqliteErrorCategory {
    fn from(value: vrcx_0_persistence::SqliteErrorCategory) -> Self {
        match value {
            vrcx_0_persistence::SqliteErrorCategory::Malformed => Self::Malformed,
            vrcx_0_persistence::SqliteErrorCategory::DiskFull => Self::DiskFull,
            vrcx_0_persistence::SqliteErrorCategory::Locked => Self::Locked,
            vrcx_0_persistence::SqliteErrorCategory::IoError => Self::IoError,
        }
    }
}

impl From<vrcx_0_persistence::Error> for AppError {
    fn from(value: vrcx_0_persistence::Error) -> Self {
        match value {
            vrcx_0_persistence::Error::Database(message) => AppError::database(message, None),
            vrcx_0_persistence::Error::Sqlite { message, category } => {
                AppError::database(message, category)
            }
            vrcx_0_persistence::Error::Io(error) => AppError::Io(error),
            vrcx_0_persistence::Error::Json(error) => AppError::Json(error),
            vrcx_0_persistence::Error::InvalidData(message) => AppError::Custom(message),
            vrcx_0_persistence::Error::Custom(message) => AppError::Custom(message),
        }
    }
}

impl From<vrcx_0_media::Error> for AppError {
    fn from(value: vrcx_0_media::Error) -> Self {
        match value {
            vrcx_0_media::Error::Io(error) => AppError::Io(error),
            vrcx_0_media::Error::Custom(message) => AppError::Custom(message),
        }
    }
}

impl From<vrcx_0_host::Error> for AppError {
    fn from(value: vrcx_0_host::Error) -> Self {
        match value {
            vrcx_0_host::Error::Io(error) => AppError::Io(error),
            vrcx_0_host::Error::Json(error) => AppError::Json(error),
            vrcx_0_host::Error::Custom(message) => AppError::Custom(message),
        }
    }
}

impl From<vrcx_0_application_core::Error> for AppError {
    fn from(value: vrcx_0_application_core::Error) -> Self {
        match value {
            vrcx_0_application_core::Error::Database(message) => AppError::database(message, None),
            vrcx_0_application_core::Error::Sqlite { message, category } => {
                AppError::database(message, category)
            }
            vrcx_0_application_core::Error::Io(error) => AppError::Io(error),
            vrcx_0_application_core::Error::Json(error) => AppError::Json(error),
            vrcx_0_application_core::Error::UpdateArtifactInvalid(message) => {
                AppError::Custom(format!("Update artifact is invalid: {message}"))
            }
            vrcx_0_application_core::Error::VrchatApi { message, .. } => AppError::Custom(message),
            vrcx_0_application_core::Error::Custom(message) => AppError::Custom(message),
        }
    }
}

impl From<vrcx_0_runtime_host::Error> for AppError {
    fn from(value: vrcx_0_runtime_host::Error) -> Self {
        match value {
            vrcx_0_runtime_host::Error::Database(message) => AppError::database(message, None),
            vrcx_0_runtime_host::Error::Sqlite { message, category } => {
                AppError::database(message, category)
            }
            vrcx_0_runtime_host::Error::Io(error) => AppError::Io(error),
            vrcx_0_runtime_host::Error::Json(error) => AppError::Json(error),
            vrcx_0_runtime_host::Error::VrchatApi { message, .. } => AppError::Custom(message),
            vrcx_0_runtime_host::Error::AuthInteractionRequired(reason)
            | vrcx_0_runtime_host::Error::AuthSessionInvalidated { reason, .. } => {
                AppError::Custom(reason)
            }
            vrcx_0_runtime_host::Error::Custom(message) => AppError::Custom(message),
        }
    }
}

impl From<vrcx_0_mcp::McpError> for AppError {
    fn from(value: vrcx_0_mcp::McpError) -> Self {
        match value {
            vrcx_0_mcp::McpError::Io(error) => AppError::Io(error),
            vrcx_0_mcp::McpError::Persistence(error) => AppError::from(error),
            vrcx_0_mcp::McpError::Application(error) => AppError::from(error),
            other => AppError::Custom(other.to_string()),
        }
    }
}

impl From<vrcx_0_assistant::AssistantError> for AppError {
    fn from(value: vrcx_0_assistant::AssistantError) -> Self {
        match value {
            vrcx_0_assistant::AssistantError::Persistence(error) => AppError::from(error),
            vrcx_0_assistant::AssistantError::Mcp(error) => AppError::from(error),
            other => AppError::Custom(other.to_string()),
        }
    }
}

impl From<vrcx_0_integrations::external_api::ExternalApiError> for AppError {
    fn from(value: vrcx_0_integrations::external_api::ExternalApiError) -> Self {
        match value {
            vrcx_0_integrations::external_api::ExternalApiError::Custom(message) => {
                AppError::Custom(message)
            }
        }
    }
}

impl From<vrcx_0_vrchat_client::HttpApiError> for AppError {
    fn from(value: vrcx_0_vrchat_client::HttpApiError) -> Self {
        match value {
            vrcx_0_vrchat_client::HttpApiError::Custom(message) => AppError::Custom(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_structured_database_error_with_sqlite_category() {
        let payload = serde_json::to_value(AppError::database(
            "opaque storage failure".into(),
            Some(vrcx_0_persistence::SqliteErrorCategory::DiskFull),
        ))
        .unwrap();

        assert_eq!(
            payload,
            serde_json::json!({
                "code": "database",
                "message": "Database error: opaque storage failure",
                "sqliteCategory": "disk_full"
            })
        );
    }

    #[test]
    fn does_not_classify_database_errors_from_display_text() {
        let payload =
            serde_json::to_value(AppError::database("database or disk is full".into(), None))
                .unwrap();

        assert_eq!(
            payload,
            serde_json::json!({
                "code": "database",
                "message": "Database error: database or disk is full"
            })
        );
    }

    #[test]
    fn omits_sqlite_category_for_unrelated_errors() {
        let payload =
            serde_json::to_value(AppError::Custom("database or disk is full".to_string())).unwrap();

        assert_eq!(
            payload,
            serde_json::json!({
                "code": "custom",
                "message": "database or disk is full"
            })
        );
    }
}

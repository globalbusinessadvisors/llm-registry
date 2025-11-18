//! Database-specific error types and conversions
//!
//! This module provides error types for database operations, including
//! connection errors, query errors, and data validation errors.

use thiserror::Error;

/// Result type alias for database operations
pub type DbResult<T> = Result<T, DbError>;

/// Database-specific errors
#[derive(Debug, Error)]
pub enum DbError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// Connection pool error
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// SQL query error
    #[error("Query error: {0}")]
    Query(String),

    /// Database migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Asset not found
    #[error("Asset not found: {0}")]
    NotFound(String),

    /// Asset already exists (duplicate)
    #[error("Asset already exists: {0}")]
    AlreadyExists(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Foreign key violation
    #[error("Foreign key violation: {0}")]
    ForeignKeyViolation(String),

    /// Unique constraint violation
    #[error("Unique constraint violation: {0}")]
    UniqueViolation(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidData(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Invalid query parameters
    #[error("Invalid query parameters: {0}")]
    InvalidQuery(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Internal database error
    #[error("Internal database error: {0}")]
    Internal(String),

    /// Domain error from core crate
    #[error("Domain error: {0}")]
    Domain(#[from] llm_registry_core::error::RegistryError),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl DbError {
    /// Check if this error is a not-found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, DbError::NotFound(_))
    }

    /// Check if this error is a constraint violation
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self,
            DbError::ConstraintViolation(_)
                | DbError::ForeignKeyViolation(_)
                | DbError::UniqueViolation(_)
        )
    }

    /// Check if this error is a duplicate/already exists error
    pub fn is_already_exists(&self) -> bool {
        matches!(self, DbError::AlreadyExists(_) | DbError::UniqueViolation(_))
    }

    /// Check if this is a transient error that could be retried
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            DbError::Connection(_) | DbError::Pool(_) | DbError::Transaction(_)
        )
    }
}

/// Convert SQLx database errors to our error type
impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => DbError::NotFound("No rows returned".to_string()),

            sqlx::Error::Database(db_err) => {
                let code = db_err.code();
                let message = db_err.message();

                // PostgreSQL error codes: https://www.postgresql.org/docs/current/errcodes-appendix.html
                match code.as_deref() {
                    Some("23505") => {
                        // unique_violation
                        DbError::UniqueViolation(message.to_string())
                    }
                    Some("23503") => {
                        // foreign_key_violation
                        DbError::ForeignKeyViolation(message.to_string())
                    }
                    Some("23514") => {
                        // check_violation
                        DbError::ConstraintViolation(message.to_string())
                    }
                    Some("23000") | Some("23001") | Some("23502") => {
                        // Various integrity constraint violations
                        DbError::ConstraintViolation(message.to_string())
                    }
                    _ => DbError::Query(message.to_string()),
                }
            }

            sqlx::Error::PoolTimedOut => DbError::Pool("Connection pool timeout".to_string()),

            sqlx::Error::PoolClosed => DbError::Pool("Connection pool closed".to_string()),

            sqlx::Error::Io(io_err) => DbError::Connection(format!("I/O error: {}", io_err)),

            sqlx::Error::Tls(tls_err) => DbError::Connection(format!("TLS error: {}", tls_err)),

            sqlx::Error::Protocol(msg) => DbError::Connection(format!("Protocol error: {}", msg)),

            sqlx::Error::TypeNotFound { type_name } => {
                DbError::InvalidData(format!("Type not found: {}", type_name))
            }

            sqlx::Error::ColumnNotFound(col) => {
                DbError::InvalidData(format!("Column not found: {}", col))
            }

            sqlx::Error::Decode(msg) => {
                DbError::Serialization(format!("Decode error: {}", msg))
            }

            sqlx::Error::Migrate(migrate_err) => {
                DbError::Migration(format!("{}", migrate_err))
            }

            _ => DbError::Internal(format!("{}", err)),
        }
    }
}

/// Convert SQLx migration errors
impl From<sqlx::migrate::MigrateError> for DbError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        DbError::Migration(format!("{}", err))
    }
}

/// Convert serde_json errors
impl From<serde_json::Error> for DbError {
    fn from(err: serde_json::Error) -> Self {
        DbError::Serialization(format!("{}", err))
    }
}

/// Convert URL parse errors
impl From<url::ParseError> for DbError {
    fn from(err: url::ParseError) -> Self {
        DbError::Configuration(format!("Invalid URL: {}", err))
    }
}

/// Convert from anyhow errors
impl From<anyhow::Error> for DbError {
    fn from(err: anyhow::Error) -> Self {
        DbError::Other(format!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let not_found = DbError::NotFound("test".to_string());
        assert!(not_found.is_not_found());
        assert!(!not_found.is_constraint_violation());

        let unique = DbError::UniqueViolation("test".to_string());
        assert!(unique.is_constraint_violation());
        assert!(unique.is_already_exists());

        let connection = DbError::Connection("test".to_string());
        assert!(connection.is_transient());
    }

    #[test]
    fn test_error_display() {
        let err = DbError::NotFound("asset-123".to_string());
        assert_eq!(err.to_string(), "Asset not found: asset-123");

        let err = DbError::UniqueViolation("duplicate key".to_string());
        assert_eq!(err.to_string(), "Unique constraint violation: duplicate key");
    }
}

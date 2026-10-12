use actix_web::http::StatusCode;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::delivery::domain::error::{ApiError, QueryError};
use crate::utils::db::DbError;

pub struct ErrorHandling {}

impl ErrorHandling {
    pub fn application_error(error_message: &str, error: Option<DbError>) -> ApiError {
        ErrorHandling::build(StatusCode::BAD_REQUEST, error_message, error)
    }

    pub fn not_found_error(error_message: &str) -> ApiError {
        ErrorHandling::build(StatusCode::NOT_FOUND, error_message, None)
    }

    pub fn internal_error(error_message: &str, error: Option<DbError>) -> ApiError {
        ErrorHandling::build(StatusCode::INTERNAL_SERVER_ERROR, error_message, error)
    }

    pub fn upstream_error(error_message: &str, error: Option<DbError>) -> ApiError {
        ErrorHandling::build(StatusCode::BAD_GATEWAY, error_message, error)
    }

    pub fn unauthorized_error() -> ApiError {
        ErrorHandling::build(
            StatusCode::UNAUTHORIZED,
            "Error: not authenticated or token expired",
            None,
        )
    }

    pub fn forbidden_error() -> ApiError {
        ErrorHandling::build(StatusCode::FORBIDDEN, "Error: resource not allowed", None)
    }

    /// Classifies an error coming out of a repository. `error_message` is the fixed message the client
    /// sees; the underlying error is only logged, except for a bad list query, whose reason is the
    /// client's own input and is returned as is.
    pub fn repository_error(error_message: &str, error: DbError) -> ApiError {
        if let Some(query_error) = error.downcast_ref::<QueryError>() {
            let message = query_error.to_string();
            return ErrorHandling::build(StatusCode::BAD_REQUEST, &message, Some(error));
        }

        let status = match error.downcast_ref::<DieselError>() {
            Some(DieselError::NotFound) => StatusCode::NOT_FOUND,
            Some(DieselError::DatabaseError(kind, _)) => match kind {
                DatabaseErrorKind::UniqueViolation => StatusCode::CONFLICT,
                DatabaseErrorKind::ForeignKeyViolation
                | DatabaseErrorKind::NotNullViolation
                | DatabaseErrorKind::CheckViolation => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ErrorHandling::build(status, error_message, Some(error))
    }

    fn build(status: StatusCode, error_message: &str, error: Option<DbError>) -> ApiError {
        ErrorHandling::log_error(status, error_message, &error);
        ApiError {
            code: status.as_u16(),
            message: String::from(error_message),
            error,
        }
    }

    fn log_error(status: StatusCode, message: &str, err: &Option<DbError>) {
        let stack = err.as_ref().map(|e| e.to_string()).unwrap_or_default();
        if status.is_server_error() {
            log::error!("{} {} {}", status.as_u16(), message, stack);
        } else {
            log::warn!("{} {} {}", status.as_u16(), message, stack);
        }
    }
}

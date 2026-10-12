use std::{error::Error, fmt};

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Error returned by services. `message` is what the client sees; `error` is the cause, only logged.
#[derive(Debug)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub error: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ApiError {}

impl ApiError {
    pub fn get_error_message(&self) -> String {
        String::from(&self.message)
    }

    pub fn get_error_code(&self) -> u16 {
        self.code
    }
}

/// Body of every error response.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ErrorPresenter {
    pub code: u16,
    pub error: String,
    pub message: String,
}

/// Error type returned by handlers and extractors; actix renders it as an `ErrorPresenter`.
#[derive(thiserror::Error, Debug)]
#[error("{error}")]
pub struct CustomErrorResponse {
    status_code: StatusCode,
    error: String,
}

impl ResponseError for CustomErrorResponse {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorPresenter {
            code: status_code.as_u16(),
            message: status_code.to_string(),
            error: self.error.clone(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

impl CustomErrorResponse {
    pub fn new(status_code: StatusCode, error: &str) -> Self {
        CustomErrorResponse {
            status_code,
            error: error.to_owned(),
        }
    }

    pub fn map_io_error(e: ApiError) -> CustomErrorResponse {
        let status_code = StatusCode::from_u16(e.get_error_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // never leak internal details on a 5xx
        let error = if status_code.is_server_error() {
            String::from("Error: an unknown error occurred")
        } else {
            e.get_error_message()
        };

        CustomErrorResponse { status_code, error }
    }

    pub fn map_system_error(e: Box<dyn Error>) -> CustomErrorResponse {
        log::error!("{}", e);
        CustomErrorResponse {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            error: String::from("Error: an unknown error occurred"),
        }
    }
}

impl From<ApiError> for CustomErrorResponse {
    fn from(e: ApiError) -> Self {
        CustomErrorResponse::map_io_error(e)
    }
}

/// A list query string the repository refuses: unknown field, criteria or a value of the wrong type.
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum QueryError {
    #[error("invalid query parameter '{0}'")]
    InvalidParam(String),
    #[error("unknown filter or sort field '{0}'")]
    UnknownField(String),
    #[error("unknown filter criteria '{0}'")]
    UnknownCriteria(String),
    #[error("invalid value '{value}' for field '{field}'")]
    InvalidValue { field: String, value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_server_error_details() {
        let e = ApiError {
            code: 500,
            message: "connection refused at 10.0.0.1".to_owned(),
            error: None,
        };
        let response = CustomErrorResponse::map_io_error(e);

        assert_eq!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.error, "Error: an unknown error occurred");
    }

    #[test]
    fn keeps_client_error_status_and_message() {
        let e = ApiError {
            code: 404,
            message: "Product not found".to_owned(),
            error: None,
        };
        let response = CustomErrorResponse::map_io_error(e);

        assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(response.error, "Product not found");
    }
}

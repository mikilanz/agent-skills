use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DefaultHttpResponseDTO {
    pub code: u16,
    pub message: String,
    pub error: bool,
}

impl DefaultHttpResponseDTO {
    pub fn success(message: &str) -> Self {
        DefaultHttpResponseDTO {
            code: StatusCode::OK.as_u16(),
            message: message.to_owned(),
            error: false,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct QueryFilter {
    pub key: String,
    pub filter: String,
    pub value: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct QueryRequest {
    pub page: i64,
    pub size: i64,
    pub sort: String,
    pub filter: Vec<QueryFilter>,
}

impl Default for QueryRequest {
    fn default() -> Self {
        QueryRequest {
            page: 0,
            size: 10,
            sort: "id,DESC".to_owned(),
            filter: Vec::new(),
        }
    }
}

impl QueryRequest {
    /// Adds a filter the client cannot override, e.g. tenant scoping. Filters are ANDed, so a client
    /// filter on the same key can only narrow the result further.
    pub fn with_filter(mut self, key: &str, filter: &str, value: &str) -> Self {
        self.filter.push(QueryFilter {
            key: key.to_owned(),
            filter: filter.to_owned(),
            value: value.to_owned(),
        });
        self
    }
}

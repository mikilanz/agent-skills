use std::path::Path;

use actix_multipart::Multipart;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWriteExt;

use crate::delivery::domain::default::{QueryFilter, QueryRequest};
use crate::delivery::domain::error::{CustomErrorResponse, QueryError};

pub const MAX_PAYLOAD_SIZE: usize = 2 * 1024 * 1024;
pub const MAX_UPLOAD_SIZE: usize = 20 * 1024 * 1024;

/// Reads a JSON body into `T`. A malformed or oversized body is a 400 / 413, never a panic.
pub async fn body_payload<T: DeserializeOwned>(mut payload: web::Payload) -> Result<T, CustomErrorResponse> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk =
            chunk.map_err(|_| CustomErrorResponse::new(StatusCode::BAD_REQUEST, "Error: cannot read request body"))?;
        if body.len() + chunk.len() > MAX_PAYLOAD_SIZE {
            return Err(CustomErrorResponse::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Error: request body too large",
            ));
        }
        body.extend_from_slice(&chunk);
    }

    serde_json::from_slice::<T>(&body)
        .map_err(|e| CustomErrorResponse::new(StatusCode::BAD_REQUEST, &format!("Error: invalid request body: {}", e)))
}

/// Caller IP. Behind a load balancer / gateway the socket peer is the proxy, so this reads
/// `Forwarded` / `X-Forwarded-For` first. Only trust it when the service is reachable through the proxy only.
pub fn client_ip(request: &HttpRequest) -> String {
    request
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_owned()
}

/// Parses the list query string: `page`, `size`, `sort=<column>,ASC|DESC` and filters
/// `<column>.<criteria>=<value>`. Other keys without a criteria are ignored. Columns and criteria
/// are validated later against the repository's whitelist.
pub fn query_transform(params: &str) -> Result<QueryRequest, QueryError> {
    let pairs = serde_urlencoded::from_str::<Vec<(String, String)>>(params)
        .map_err(|_| QueryError::InvalidParam(params.to_owned()))?;

    let mut query_request = QueryRequest::default();
    let invalid = |field: &str, value: &str| QueryError::InvalidValue {
        field: field.to_owned(),
        value: value.to_owned(),
    };

    for (key, value) in pairs {
        match key.as_str() {
            "page" => query_request.page = value.parse::<i64>().map_err(|_| invalid("page", &value))?,
            "size" => query_request.size = value.parse::<i64>().map_err(|_| invalid("size", &value))?,
            "sort" => query_request.sort = value,
            _ => {
                if let Some((column, criteria)) = key.split_once('.') {
                    query_request.filter.push(QueryFilter {
                        key: column.to_owned(),
                        filter: criteria.to_owned(),
                        value,
                    });
                }
            }
        }
    }

    Ok(query_request)
}

/// Saves every file of a multipart upload into `dir` and returns the saved paths. The client's file
/// name is reduced to its last component and prefixed with a uuid, so it cannot escape `dir` or
/// overwrite another upload. The caller deletes the files when done.
pub async fn multipart_to_files(mut payload: Multipart, dir: &str) -> Result<Vec<String>, CustomErrorResponse> {
    let bad_request = |message: &str| CustomErrorResponse::new(StatusCode::BAD_REQUEST, message);
    let mut filepaths: Vec<String> = Vec::new();

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|_| bad_request("Error: invalid multipart payload"))?;

        let filename = field
            .content_disposition()
            .and_then(|content_disposition| content_disposition.get_filename())
            .and_then(|name| Path::new(name).file_name())
            .and_then(|name| name.to_str())
            .map(|name| name.to_owned());

        let Some(filename) = filename else {
            continue; // a plain form field, not a file
        };

        let filepath = format!("{}/{}-{}", dir, uuid::Uuid::new_v4(), filename);
        let mut file = tokio::fs::File::create(&filepath).await.map_err(|e| {
            log::error!("cannot create {}: {}", filepath, e);
            CustomErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Error: cannot store upload")
        })?;

        let mut written = 0usize;
        while let Some(chunk) = field.next().await {
            let data = chunk.map_err(|_| bad_request("Error: invalid multipart payload"))?;
            written += data.len();
            if written > MAX_UPLOAD_SIZE {
                let _ = tokio::fs::remove_file(&filepath).await;
                return Err(CustomErrorResponse::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Error: file too large",
                ));
            }
            file.write_all(&data).await.map_err(|e| {
                log::error!("cannot write {}: {}", filepath, e);
                CustomErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Error: cannot store upload")
            })?;
        }

        filepaths.push(filepath);
    }

    Ok(filepaths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paging_sort_and_filters() {
        let query =
            query_transform("page=2&size=5&sort=name,ASC&name.includes=a%20b&status.in=active,inactive&_=123").unwrap();

        assert_eq!(query.page, 2);
        assert_eq!(query.size, 5);
        assert_eq!(query.sort, "name,ASC");
        assert_eq!(
            query.filter,
            vec![
                QueryFilter {
                    key: "name".into(),
                    filter: "includes".into(),
                    value: "a b".into()
                },
                QueryFilter {
                    key: "status".into(),
                    filter: "in".into(),
                    value: "active,inactive".into()
                },
            ]
        );
    }

    #[test]
    fn defaults_when_empty() {
        assert_eq!(query_transform("").unwrap(), QueryRequest::default());
    }

    #[test]
    fn client_ip_prefers_forwarded_for() {
        let request = actix_web::test::TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.7, 10.0.0.1"))
            .to_http_request();
        assert_eq!(client_ip(&request), "203.0.113.7");
    }

    #[test]
    fn rejects_non_numeric_paging() {
        assert!(query_transform("page=abc").is_err());
    }
}

use reqwest::header::CONTENT_TYPE;
use reqwest::multipart;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::application::error::error_handling::ErrorHandling;
use crate::delivery::domain::error::ApiError;
use crate::delivery::domain::request_domain::RequestOptionDomain;

/// Outbound HTTP. Uses the shared `reqwest::Client` from `GeneralModules` (connection pooling,
/// 30s timeout). The body is JSON unless the headers set `Content-Type:
/// application/x-www-form-urlencoded`. Non-2xx responses become a 502 `ApiError`; the upstream body
/// is only logged.
pub struct RequestService<'a> {
    client: &'a reqwest::Client,
}

impl<'a> RequestService<'a> {
    pub fn new(client: &'a reqwest::Client) -> Self {
        RequestService { client }
    }

    pub async fn request(&self, options: RequestOptionDomain) -> Result<String, ApiError> {
        let method = Method::from_bytes(options.method.to_uppercase().as_bytes())
            .map_err(|e| ErrorHandling::internal_error("Invalid outbound request method", Some(e.into())))?;

        let is_form = options.headers.iter().any(|(key, value)| {
            key.eq_ignore_ascii_case(CONTENT_TYPE.as_str()) && value.starts_with("application/x-www-form-urlencoded")
        });

        let mut builder = self.client.request(method.clone(), &options.url);
        for (key, value) in &options.headers {
            builder = builder.header(key, value);
        }
        if !options.payload.is_empty() {
            if is_form {
                // payload is a flat JSON object, sent as key=value&...
                let fields = serde_json::from_str::<serde_json::Map<String, Value>>(&options.payload)
                    .map_err(|e| ErrorHandling::internal_error("Form payload must be a JSON object", Some(e.into())))?;
                let form = fields
                    .into_iter()
                    .map(|(key, value)| match value {
                        Value::String(text) => (key, text),
                        other => (key, other.to_string()),
                    })
                    .collect::<Vec<(String, String)>>();
                let encoded = serde_urlencoded::to_string(&form)
                    .map_err(|e| ErrorHandling::internal_error("Cannot encode form payload", Some(e.into())))?;
                builder = builder.body(encoded);
            } else {
                builder = builder.header(CONTENT_TYPE, "application/json").body(options.payload);
            }
        }

        let response = builder
            .send()
            .await
            .map_err(|e| ErrorHandling::upstream_error("Upstream service unreachable", Some(e.into())))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ErrorHandling::upstream_error("Cannot read upstream response", Some(e.into())))?;

        if !status.is_success() {
            log::error!("{} {} responded {}: {}", method, options.url, status, body);
            return Err(ErrorHandling::upstream_error(
                &format!("Upstream service responded {}", status),
                None,
            ));
        }

        Ok(body)
    }

    pub async fn request_json<T: DeserializeOwned>(&self, options: RequestOptionDomain) -> Result<T, ApiError> {
        let body = self.request(options).await?;

        serde_json::from_str::<T>(&body)
            .map_err(|e| ErrorHandling::upstream_error("Cannot parse upstream response", Some(e.into())))
    }

    /// Multipart upload. Never set Content-Type yourself: reqwest adds it with the boundary.
    pub async fn file_upload(
        &self,
        url: &str,
        field_name: &str,
        buffer: Vec<u8>,
        filename: String,
        mime: &str,
    ) -> Result<String, ApiError> {
        let part = multipart::Part::bytes(buffer)
            .file_name(filename)
            .mime_str(mime)
            .map_err(|e| ErrorHandling::internal_error("Invalid upload mime type", Some(e.into())))?;
        let form = multipart::Form::new().part(field_name.to_owned(), part);

        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ErrorHandling::upstream_error("Upload service unreachable", Some(e.into())))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ErrorHandling::upstream_error("Cannot read upload response", Some(e.into())))?;

        if !status.is_success() {
            log::error!("POST {} responded {}: {}", url, status, body);
            return Err(ErrorHandling::upstream_error(
                &format!("Upload service responded {}", status),
                None,
            ));
        }

        Ok(body)
    }
}

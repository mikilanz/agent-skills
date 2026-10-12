use std::future::{ready, Ready};
use std::ops::Deref;

use actix_web::dev::Payload;
use actix_web::http::StatusCode;
use actix_web::{FromRequest, HttpRequest};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;

use crate::delivery::domain::error::CustomErrorResponse;
use crate::delivery::interfaces::general_interface::Claims;

const USERINFO_HEADERS: [&str; 2] = ["x-endpoint-api-userinfo", "x-apigateway-api-userinfo"];

/// Extractor for the user claims the API gateway (Cloud Endpoints / API Gateway) forwards after it
/// has validated the JWT. Add `claim: ClaimGuardMiddleware` to a handler to require it.
pub struct ClaimGuardMiddleware {
    pub userinfo: Value,
    pub claims: Claims,
}

impl Deref for ClaimGuardMiddleware {
    type Target = Claims;

    fn deref(&self) -> &Claims {
        &self.claims
    }
}

impl FromRequest for ClaimGuardMiddleware {
    type Error = CustomErrorResponse;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let header = USERINFO_HEADERS
            .iter()
            .find_map(|name| req.headers().get(*name))
            .and_then(|value| value.to_str().ok());

        let result = match header {
            Some(encoded) => decode_userinfo(encoded)
                .map(|(userinfo, claims)| ClaimGuardMiddleware { userinfo, claims })
                .ok_or_else(|| CustomErrorResponse::new(StatusCode::UNAUTHORIZED, "Error: invalid user info")),
            None => Err(CustomErrorResponse::new(
                StatusCode::UNAUTHORIZED,
                "Error: not authenticated or token expired",
            )),
        };

        ready(result)
    }
}

/// Decodes the base64(url) user info header. The gateway may nest the token payload as a JSON string
/// (or object) under `claims`.
pub fn decode_userinfo(encoded: &str) -> Option<(Value, Claims)> {
    let trimmed = encoded.trim().trim_end_matches('=');
    let bytes = URL_SAFE_NO_PAD
        .decode(trimmed)
        .or_else(|_| STANDARD.decode(encoded.trim()))
        .ok()?;

    let mut userinfo = serde_json::from_slice::<Value>(&bytes).ok()?;

    match userinfo.get("claims") {
        Some(Value::String(nested)) => userinfo = serde_json::from_str::<Value>(nested).ok()?,
        Some(Value::Object(_)) => userinfo = userinfo["claims"].take(),
        _ => (),
    }

    let claims = serde_json::from_value::<Claims>(userinfo.clone()).ok()?;

    Some((userinfo, claims))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn encode(value: &Value) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap())
    }

    #[actix_web::test]
    async fn extracts_claims_from_gateway_header() {
        let header = encode(&serde_json::json!({ "companyCode": "ACME", "role": "admin", "sub": "u-1" }));
        let req = TestRequest::default()
            .insert_header(("x-apigateway-api-userinfo", header))
            .to_http_request();

        let claim = ClaimGuardMiddleware::extract(&req).await.unwrap();

        assert_eq!(claim.company_code, "ACME");
        assert_eq!(claim.role, "admin");
    }

    #[actix_web::test]
    async fn reads_nested_claims_string() {
        let nested = serde_json::json!({ "companyCode": "ACME", "role": "user" }).to_string();
        let header = STANDARD.encode(serde_json::to_vec(&serde_json::json!({ "claims": nested })).unwrap());

        let (_, claims) = decode_userinfo(&header).unwrap();

        assert_eq!(claims.company_code, "ACME");
        assert_eq!(claims.role, "user");
    }

    #[actix_web::test]
    async fn rejects_missing_or_invalid_header() {
        let req = TestRequest::default().to_http_request();
        assert!(ClaimGuardMiddleware::extract(&req).await.is_err());

        let req = TestRequest::default()
            .insert_header(("x-endpoint-api-userinfo", "not-base64-json"))
            .to_http_request();
        assert!(ClaimGuardMiddleware::extract(&req).await.is_err());
    }
}

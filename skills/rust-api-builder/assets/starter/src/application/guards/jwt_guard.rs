use std::future::{ready, Ready};
use std::ops::Deref;

use actix_web::dev::Payload;
use actix_web::http::StatusCode;
use actix_web::{FromRequest, HttpRequest};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::config::config_service::ConfigService;
use crate::delivery::domain::error::CustomErrorResponse;
use crate::delivery::interfaces::general_interface::Claims;

/// Extractor that verifies `Authorization: Bearer <jwt>` itself (RS256, public key from `JWKS_N` /
/// `JWKS_E`, the `n` and `e` of the issuer's JWKS key). Use it for routes the API gateway does not
/// authenticate; gateway routes use `ClaimGuardMiddleware`. Both yield the same `Claims`.
pub struct JwtMiddleware {
    pub claims: Claims,
}

impl Deref for JwtMiddleware {
    type Target = Claims;

    fn deref(&self) -> &Claims {
        &self.claims
    }
}

impl FromRequest for JwtMiddleware {
    type Error = CustomErrorResponse;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let unauthorized =
            || CustomErrorResponse::new(StatusCode::UNAUTHORIZED, "Error: not authenticated or token expired");

        let token = req
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        let Some(token) = token else {
            return ready(Err(unauthorized()));
        };

        let config_service = ConfigService {};
        let decoding_key = match DecodingKey::from_rsa_components(
            &config_service.get_env_or("JWKS_N", ""),
            &config_service.get_env_or("JWKS_E", ""),
        ) {
            Ok(key) => key,
            Err(e) => {
                log::error!("JWKS_N / JWKS_E missing or invalid: {}", e);
                return ready(Err(unauthorized()));
            }
        };

        let mut validation = Validation::new(Algorithm::RS256);
        let audience = config_service.get_env_or("JWT_AUDIENCE", "");
        if audience.is_empty() {
            validation.validate_aud = false;
        } else {
            validation.set_audience(&[audience]);
        }

        let result = decode::<Claims>(token, &decoding_key, &validation)
            .map(|data| JwtMiddleware { claims: data.claims })
            .map_err(|e| {
                log::warn!("rejected token: {}", e);
                unauthorized()
            });

        ready(result)
    }
}

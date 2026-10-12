use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ROLE_SUPERADMIN: &str = "superadmin";
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_EMPLOYEE: &str = "employee";
pub const ROLE_USER: &str = "user";

/// User claims: forwarded by the API gateway in `x-endpoint-api-userinfo` / `x-apigateway-api-userinfo`
/// (`ClaimGuardMiddleware`), or decoded from the bearer token (`JwtMiddleware`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Claims {
    #[serde(rename = "companyKey")]
    pub company_key: String,
    #[serde(rename = "companyCode")]
    pub company_code: String,
    pub role: String,
    pub scope: String,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub phone: String,
    pub phone_verified: bool,
    pub aud: Value,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
    pub jti: String,
    pub sub: String,
}

impl Claims {
    /// The company a request may act on: superadmin may pick any company (None = all companies),
    /// everyone else is pinned to the company in their claims.
    pub fn scoped_company_code(&self, requested: Option<String>) -> Option<String> {
        if self.role == ROLE_SUPERADMIN {
            requested.filter(|code| !code.is_empty())
        } else {
            Some(self.company_code.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_non_superadmin_to_own_company() {
        let admin = Claims {
            role: ROLE_ADMIN.to_owned(),
            company_code: "ACME".to_owned(),
            ..Default::default()
        };
        let superadmin = Claims {
            role: ROLE_SUPERADMIN.to_owned(),
            ..Default::default()
        };

        assert_eq!(
            admin.scoped_company_code(Some("OTHER".to_owned())),
            Some("ACME".to_owned())
        );
        assert_eq!(
            superadmin.scoped_company_code(Some("OTHER".to_owned())),
            Some("OTHER".to_owned())
        );
        assert_eq!(superadmin.scoped_company_code(None), None);
    }
}

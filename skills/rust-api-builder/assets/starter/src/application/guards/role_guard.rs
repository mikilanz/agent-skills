use actix_web::http::StatusCode;

use crate::delivery::domain::error::CustomErrorResponse;
use crate::delivery::interfaces::general_interface::Claims;

/// Returns 403 unless the caller's role is one of `roles`. Call it first in a handler:
/// `role_guard(&claim, &[ROLE_SUPERADMIN, ROLE_ADMIN])?;`
pub fn role_guard(claims: &Claims, roles: &[&str]) -> Result<(), CustomErrorResponse> {
    if roles.iter().any(|role| *role == claims.role) {
        Ok(())
    } else {
        Err(CustomErrorResponse::new(
            StatusCode::FORBIDDEN,
            "Error: resource not allowed",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::interfaces::general_interface::{ROLE_ADMIN, ROLE_SUPERADMIN, ROLE_USER};

    #[test]
    fn allows_listed_roles_only() {
        let admin = Claims {
            role: ROLE_ADMIN.to_owned(),
            ..Default::default()
        };
        let user = Claims {
            role: ROLE_USER.to_owned(),
            ..Default::default()
        };

        assert!(role_guard(&admin, &[ROLE_SUPERADMIN, ROLE_ADMIN]).is_ok());
        assert!(role_guard(&user, &[ROLE_SUPERADMIN, ROLE_ADMIN]).is_err());
    }
}

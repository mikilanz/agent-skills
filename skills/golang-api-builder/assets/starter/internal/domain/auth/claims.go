package auth

// Claims is the decoded user info the API gateway forwards in the
// x-apigateway-api-userinfo / x-endpoint-api-userinfo header.
type Claims struct {
	CompanyKey    string `json:"companyKey"`
	CompanyCode   string `json:"companyCode"`
	Role          string `json:"role"`
	Scope         string `json:"scope"`
	Username      string `json:"username"`
	Email         string `json:"email"`
	EmailVerified bool   `json:"email_verified"`
	Phone         string `json:"phone"`
	PhoneVerified bool   `json:"phone_verified"`
	Aud           string `json:"aud"`
	Exp           int64  `json:"exp"`
	Jti           string `json:"jti"`
	Iat           int64  `json:"iat"`
	Iss           string `json:"iss"`
	Sub           string `json:"sub"`
}

const (
	RoleSuperadmin = "superadmin"
	RoleAdmin      = "admin"
	RoleEmployee   = "employee"
	RoleUser       = "user"
)

// ScopedCompanyCode returns the tenant a request may act on: superadmin may pick any
// company, everyone else is pinned to the company in their claims.
func (c Claims) ScopedCompanyCode(requested string) string {
	if c.Role == RoleSuperadmin {
		return requested
	}
	return c.CompanyCode
}

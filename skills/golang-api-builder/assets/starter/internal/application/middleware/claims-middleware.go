package middleware

import (
	"encoding/base64"
	"encoding/json"
	"net/http"

	domain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/domain/general"

	"github.com/gin-gonic/gin"
)

const userInfoKey = "userInfo"

// ClaimsMiddleware trusts the API gateway: the gateway validates the JWT and forwards
// its payload base64-encoded in x-apigateway-api-userinfo (API Gateway) or
// x-endpoint-api-userinfo (Cloud Endpoints / ESPv2). Never expose this service
// directly to the internet without the gateway in front of it.
func ClaimsMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		userInfoHeader := c.GetHeader("x-apigateway-api-userinfo")
		if userInfoHeader == "" {
			userInfoHeader = c.GetHeader("x-endpoint-api-userinfo")
		}
		if userInfoHeader == "" {
			c.AbortWithStatusJSON(http.StatusUnauthorized, general.DefaultHttpResponse{
				StatusCode: http.StatusUnauthorized,
				Message:    "Missing user credentials",
				Ok:         false,
			})
			return
		}

		decodedHeaderInfo, ok := decodeBase64(userInfoHeader)
		if !ok {
			c.AbortWithStatusJSON(http.StatusUnauthorized, general.DefaultHttpResponse{
				StatusCode: http.StatusUnauthorized,
				Message:    "Invalid user credentials",
				Ok:         false,
			})
			return
		}

		var claims domain.Claims
		if err := json.Unmarshal(decodedHeaderInfo, &claims); err != nil {
			c.AbortWithStatusJSON(http.StatusUnauthorized, general.DefaultHttpResponse{
				StatusCode: http.StatusUnauthorized,
				Message:    "Invalid user credentials information",
				Ok:         false,
			})
			return
		}

		c.Set(userInfoKey, claims)
		c.Next()
	}
}

// GetClaims returns the claims ClaimsMiddleware stored on the context.
func GetClaims(c *gin.Context) (domain.Claims, bool) {
	userInfo, exists := c.Get(userInfoKey)
	if !exists {
		return domain.Claims{}, false
	}
	claims, ok := userInfo.(domain.Claims)
	return claims, ok
}

// gateways differ in padding and alphabet, so try every base64 variant
func decodeBase64(value string) ([]byte, bool) {
	for _, encoding := range []*base64.Encoding{
		base64.RawStdEncoding,
		base64.StdEncoding,
		base64.RawURLEncoding,
		base64.URLEncoding,
	} {
		if decoded, err := encoding.DecodeString(value); err == nil {
			return decoded, true
		}
	}
	return nil, false
}

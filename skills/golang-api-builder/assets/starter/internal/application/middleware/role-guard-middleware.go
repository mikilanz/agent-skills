package middleware

import (
	"net/http"
	"slices"

	"go-api-starter/internal/domain/general"

	"github.com/gin-gonic/gin"
)

func RoleGuard(allowedRoles ...string) gin.HandlerFunc {
	return func(c *gin.Context) {
		claims, ok := GetClaims(c)
		if !ok {
			c.AbortWithStatusJSON(http.StatusUnauthorized, general.DefaultHttpResponse{
				StatusCode: http.StatusUnauthorized,
				Message:    "Missing user credentials",
				Ok:         false,
			})
			return
		}

		if slices.Contains(allowedRoles, claims.Role) {
			c.Next()
			return
		}

		c.AbortWithStatusJSON(http.StatusForbidden, general.DefaultHttpResponse{
			StatusCode: http.StatusForbidden,
			Message:    "Unauthorized role",
			Ok:         false,
		})
	}
}

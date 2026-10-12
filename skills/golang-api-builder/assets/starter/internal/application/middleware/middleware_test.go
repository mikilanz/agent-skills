package middleware

import (
	"encoding/base64"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"testing"

	domain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/utils"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"
)

func newTestRouter(handlers ...gin.HandlerFunc) *gin.Engine {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.GET("/test", handlers...)
	return router
}

func TestClaimsMiddlewareAndRoleGuard(t *testing.T) {
	var got domain.Claims
	router := newTestRouter(ClaimsMiddleware(), RoleGuard(domain.RoleAdmin), func(c *gin.Context) {
		got, _ = GetClaims(c)
		c.Status(http.StatusOK)
	})

	header := base64.RawURLEncoding.EncodeToString([]byte(`{"companyCode":"ACME","role":"admin","username":"jane"}`))

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	req.Header.Set("x-endpoint-api-userinfo", header)
	router.ServeHTTP(w, req)
	assert.Equal(t, http.StatusOK, w.Code)
	assert.Equal(t, "ACME", got.CompanyCode)

	w = httptest.NewRecorder()
	router.ServeHTTP(w, httptest.NewRequest(http.MethodGet, "/test", nil))
	assert.Equal(t, http.StatusUnauthorized, w.Code)

	userHeader := base64.StdEncoding.EncodeToString([]byte(`{"role":"user"}`))
	w = httptest.NewRecorder()
	req = httptest.NewRequest(http.MethodGet, "/test", nil)
	req.Header.Set("x-apigateway-api-userinfo", userHeader)
	router.ServeHTTP(w, req)
	assert.Equal(t, http.StatusForbidden, w.Code)
}

func TestQueryPaginationMiddleware(t *testing.T) {
	var got utils.QueryRequest
	router := newTestRouter(QueryPaginationMiddleware(slog.Default()), func(c *gin.Context) {
		got = GetQueryRequest(c)
		c.Status(http.StatusOK)
	})

	w := httptest.NewRecorder()
	router.ServeHTTP(w, httptest.NewRequest(http.MethodGet, "/test?page=2&size=5&sort=createdAt,desc&status.in=a,%20b&name.bogus=x", nil))

	assert.Equal(t, 2, got.Page)
	assert.Equal(t, 5, got.Size)
	assert.Equal(t, []utils.SortData{{Key: "createdAt", Sort: utils.SortOrderDesc}}, got.Sort)
	assert.Equal(t, []utils.QueryFilter{{Key: "status", Value: []string{"a", "b"}, Filter: utils.FilterCriteriaIn}}, got.Filters)
}

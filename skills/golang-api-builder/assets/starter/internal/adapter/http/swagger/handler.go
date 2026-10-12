package swagger

import (
	"log/slog"
	"net/http"

	"go-api-starter/api/docs"
	"go-api-starter/internal/config"

	"github.com/gin-gonic/gin"
	swaggerFiles "github.com/swaggo/files"
	ginSwagger "github.com/swaggo/gin-swagger"
)

type Handler struct {
	logger *slog.Logger
}

func NewHandler(logger *slog.Logger) *Handler {
	return &Handler{logger: logger}
}

func (h *Handler) RegisterRoutes(rg *gin.RouterGroup) {
	rg.GET("swagger/*any", h.customWrapHandler)
}

func (h *Handler) customWrapHandler(c *gin.Context) {
	cfg := config.GetConfig()

	if c.Request.RequestURI == "/swagger/" {
		c.Redirect(http.StatusFound, "/swagger/index.html")
		return
	}

	docs.SwaggerInfo.Host = c.Request.Host
	docs.SwaggerInfo.Version = cfg.APP_VERSION
	ginSwagger.CustomWrapHandler(
		&ginSwagger.Config{URL: "/swagger/doc.json"},
		swaggerFiles.Handler,
	)(c)
}

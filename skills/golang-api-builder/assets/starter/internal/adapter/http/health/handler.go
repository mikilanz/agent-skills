package health

import (
	"context"
	"log/slog"
	"net/http"
	"time"

	"go-api-starter/internal/domain/general"

	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/redis/go-redis/v9"
)

type Handler struct {
	db     *pgxpool.Pool
	rdb    *redis.Client
	logger *slog.Logger
}

func NewHandler(
	db *pgxpool.Pool,
	rdb *redis.Client,
	logger *slog.Logger,
) *Handler {
	return &Handler{
		db:     db,
		rdb:    rdb,
		logger: logger,
	}
}

func (h *Handler) RegisterRoutes(rg *gin.RouterGroup) {
	rg.GET("health", h.health)
}

// Health check
//
//	@Summary	Health check
//	@Schemes
//	@Description	Checks the database and Redis connections
//	@Tags			Health
//	@Produce		json
//	@Success		200	{object}	general.DefaultHttpResponse
//	@Failure		503	{object}	general.DefaultHttpResponse
//	@Router			/health [get]
func (h *Handler) health(c *gin.Context) {
	ctx, cancel := context.WithTimeout(c.Request.Context(), 3*time.Second)
	defer cancel()

	if err := h.db.Ping(ctx); err != nil {
		h.logger.Error("health check failed: database", slog.String("error", err.Error()))
		c.JSON(http.StatusServiceUnavailable, general.DefaultHttpResponse{
			StatusCode: http.StatusServiceUnavailable,
			Message:    "database unavailable",
			Ok:         false,
		})
		return
	}

	if err := h.rdb.Ping(ctx).Err(); err != nil {
		h.logger.Error("health check failed: redis", slog.String("error", err.Error()))
		c.JSON(http.StatusServiceUnavailable, general.DefaultHttpResponse{
			StatusCode: http.StatusServiceUnavailable,
			Message:    "redis unavailable",
			Ok:         false,
		})
		return
	}

	c.JSON(http.StatusOK, general.DefaultHttpResponse{
		StatusCode: http.StatusOK,
		Message:    "ok",
		Ok:         true,
	})
}

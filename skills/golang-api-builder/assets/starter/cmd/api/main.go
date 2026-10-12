package main

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"
	"time"

	"go-api-starter/internal/application/middleware"
	"go-api-starter/internal/config"
	"go-api-starter/internal/utils"

	healthHandler "go-api-starter/internal/adapter/http/health"
	swaggerHandler "go-api-starter/internal/adapter/http/swagger"

	"github.com/gin-contrib/cors"
	"github.com/gin-contrib/graceful"
	ginSlog "github.com/gin-contrib/slog"
	"github.com/gin-gonic/gin"
	"github.com/golang-migrate/migrate/v4"
	"github.com/golang-migrate/migrate/v4/database/pgx/v5"
	_ "github.com/golang-migrate/migrate/v4/source/file"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/redis/go-redis/v9"
)

//	@title			Go API Starter
//	@version		1.0.0
//	@description	Collection of Go API Starter endpoints.

// @securityDefinitions.apikey	BearerAuth
// @in							header
// @name						Authorization
// @description				Client-provided user JWT containing information to be extracted from
func main() {
	// Load configuration
	config.InitConfig()
	cfg := config.GetConfig()

	// Initialize utilities
	log := utils.InitLogging(cfg)
	gin.SetMode(cfg.GIN_MODE)

	slog.Info("Starting Go API Starter...")

	// Setup database connection
	ctx := context.Background()
	dbpool, err := pgxpool.New(ctx, cfg.DATABASE_URL)
	if err != nil {
		slog.Error("Failed to connect to database", "error", err)
		os.Exit(1)
	}
	defer dbpool.Close()

	if cfg.AUTO_DATABASE_MIGRATION == "ON" {
		if err := runMigrations(cfg.DATABASE_URL); err != nil {
			slog.Error("Failed to run database migrations", "error", err)
			os.Exit(1)
		}
	}

	// Setup Redis cache connection
	redisOptions, err := redis.ParseURL(cfg.REDIS_URL)
	if err != nil {
		slog.Error("Failed to parse Redis URL", "error", err)
		os.Exit(1)
	}
	redisOptions.DB = cfg.REDIS_DATABASE
	redisOptions.MinIdleConns = 10
	redisOptions.PoolSize = 50
	redisOptions.MaxRetries = 3
	redisOptions.DialTimeout = 5 * time.Second
	redisOptions.ReadTimeout = 3 * time.Second
	redisOptions.WriteTimeout = 3 * time.Second
	rdb := redis.NewClient(redisOptions)
	defer rdb.Close()

	// Dependency Injection: repositories -> services -> handlers
	var (
		healthHandler  = healthHandler.NewHandler(dbpool, rdb, log)
		swaggerHandler = swaggerHandler.NewHandler(log)
	)

	// Setup HTTP server
	appPort := fmt.Sprintf(":%v", cfg.APP_PORT)
	// gin.New, not graceful.Default: Default adds gin.Logger and a second Recovery
	router, err := graceful.New(gin.New(), graceful.WithAddr(appPort))
	if err != nil {
		slog.Error("Failed to start server", "error", err)
		os.Exit(1)
	}
	defer router.Close()

	// Middlewares
	router.Use(ginSlog.SetLogger(
		ginSlog.WithLogger(func(c *gin.Context, _ *slog.Logger) *slog.Logger {
			return log
		}),
		ginSlog.WithSkipPath([]string{"/health"}),
	))
	router.Use(gin.Recovery())
	router.Use(cors.New(cors.Config{
		AllowAllOrigins:  true,
		AllowMethods:     []string{"GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"},
		AllowHeaders:     []string{"Origin", "Content-Type", "Authorization"},
		ExposeHeaders:    []string{"Content-Type", "Authorization", "Cache-Control", "Connection", "Accept", "Origin", "x-total-count"},
		AllowCredentials: true,
		MaxAge:           12 * time.Hour,
	}))

	// Routes
	apiV1 := router.Group("api/v1")
	api := router.Group("")
	swaggerHandler.RegisterRoutes(api)
	healthHandler.RegisterRoutes(api)
	apiV1.Use(middleware.ClaimsMiddleware(), middleware.QueryPaginationMiddleware(log))
	{
		// featureHandler.RegisterRoutes(apiV1)
	}

	// Start HTTP server with graceful shutdown
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	slog.Info("Server is listening", "port", appPort)
	if err := router.RunWithContext(ctx); err != nil && !errors.Is(err, context.Canceled) {
		slog.Error("Server stopped with error", "error", err)
		os.Exit(1)
	}
	slog.Info("Server shutdown gracefully")
}

// runMigrations applies internal/database/migrations (path relative to the working directory).
func runMigrations(databaseURL string) error {
	db, err := sql.Open("pgx", databaseURL)
	if err != nil {
		return fmt.Errorf("acquire database connection: %w", err)
	}
	defer db.Close()

	driver, err := pgx.WithInstance(db, &pgx.Config{})
	if err != nil {
		return fmt.Errorf("create migration driver: %w", err)
	}

	m, err := migrate.NewWithDatabaseInstance("file://internal/database/migrations", "pgx", driver)
	if err != nil {
		return fmt.Errorf("create migration instance: %w", err)
	}

	if err := m.Up(); err != nil && !errors.Is(err, migrate.ErrNoChange) {
		return err
	}
	return nil
}

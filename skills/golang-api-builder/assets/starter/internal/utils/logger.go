package utils

import (
	"log/slog"
	"os"
	"strings"

	"go-api-starter/internal/config"
)

func InitLogging(cfg *config.Config) *slog.Logger {
	levelStr := strings.ToUpper(cfg.LOG_LEVEL)
	var level slog.Level
	switch levelStr {
	case "DEBUG":
		level = slog.LevelDebug
	case "WARN", "WARNING":
		level = slog.LevelWarn
	case "ERROR":
		level = slog.LevelError
	default:
		level = slog.LevelInfo
	}

	handler := slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: level})
	logger := slog.New(handler)
	// package-level slog.Info/Error calls use the same JSON handler
	slog.SetDefault(logger)

	logger.Info("Logger initialized", "level", level)
	return logger
}

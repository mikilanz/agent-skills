package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"go-api-starter/internal/config"
	"go-api-starter/internal/utils"
)

func main() {
	// Load configuration
	config.InitConfig()
	cfg := config.GetConfig()
	log := utils.InitLogging(cfg)

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	var wg sync.WaitGroup
	wg.Go(func() { RunWorker(ctx, cfg, log) })

	<-ctx.Done()
	slog.Info("Got stop signal, shutting down worker gracefully")
	// consumers must return once ctx is cancelled
	wg.Wait()
	os.Exit(0)
}

func RunWorker(ctx context.Context, cfg *config.Config, log *slog.Logger) {
	log.Info("Worker started")

	// Worker logic goes here: consume a queue or poll on a ticker until ctx.Done()
	<-ctx.Done()
	log.Info("Worker stopped")
}

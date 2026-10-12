package config

import (
	"log"
	"os"
	"strconv"
	"sync"

	"github.com/joho/godotenv"
)

// Config contains the configuration settings
type Config struct {
	// General
	APP_PORT    string
	APP_VERSION string
	GIN_MODE    string

	// Logger
	LOG_LEVEL string

	// Redis
	REDIS_URL      string
	REDIS_DATABASE int

	// Database
	DATABASE_URL            string
	AUTO_DATABASE_MIGRATION string

	// External Services
}

var (
	instance *Config
	once     sync.Once
)

// Initializes the global config instance
func InitConfig() {
	once.Do(func() {
		if err := godotenv.Load(); err != nil {
			log.Printf("Could not load .env file (%v), using environment variables.\n", err)
		}

		instance = &Config{
			APP_PORT:                getEnvOrDefault("APP_PORT", "8080"),
			APP_VERSION:             getEnvOrFatal("APP_VERSION"),
			GIN_MODE:                getEnvOrDefault("GIN_MODE", "debug"),
			LOG_LEVEL:               getEnvOrDefault("LOG_LEVEL", "INFO"),
			REDIS_URL:               getEnvOrFatal("REDIS_URL"),
			REDIS_DATABASE:          getEnvIntOrDefault("REDIS_DATABASE", 0),
			DATABASE_URL:            getEnvOrFatal("DATABASE_URL"),
			AUTO_DATABASE_MIGRATION: getEnvOrDefault("AUTO_DATABASE_MIGRATION", "OFF"),
		}
		log.Println("Configuration has been successfully initialized.")
	})
}

// Provides global access to the immutable Config instance
func GetConfig() *Config {
	if instance == nil {
		log.Fatal("Configuration not initialized. Call InitConfig() first.")
	}
	return instance
}

// Helper: returns value or logs fatal if missing
func getEnvOrFatal(key string) string {
	val := os.Getenv(key)
	if val == "" {
		log.Fatalf("Failed to read '%s': no valid value found in allowed sources.", key)
	}
	return val
}

// Helper: returns value or default if missing
func getEnvOrDefault(key, def string) string {
	val := os.Getenv(key)
	if val == "" {
		return def
	}
	return val
}

// Helper: returns integer value, default if missing, logs fatal if not an integer
func getEnvIntOrDefault(key string, def int) int {
	val := os.Getenv(key)
	if val == "" {
		return def
	}
	parsed, err := strconv.Atoi(val)
	if err != nil {
		log.Fatalf("Failed to read '%s': must be an integer.", key)
	}
	return parsed
}

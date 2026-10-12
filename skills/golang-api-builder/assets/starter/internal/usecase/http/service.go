package http

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"time"

	httpAdapter "go-api-starter/internal/domain/general"
)

// Service is the single outbound HTTP client. Other services call external APIs
// through it so timeouts and request logging stay in one place.
type Service struct {
	httpClient *http.Client
	logger     *slog.Logger
}

func NewService(logger *slog.Logger) *Service {
	return &Service{
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
		logger: logger,
	}
}

// MakeRequest sends the request and returns the raw response. The caller must close res.Body.
func (s *Service) MakeRequest(ctx context.Context, options *httpAdapter.HttpOptions) (*http.Response, error) {
	req, err := http.NewRequestWithContext(ctx, options.Method, options.URL, options.Body)
	if err != nil {
		s.logger.Error("Failed to create new request", "error", err, "method", options.Method, "url", options.URL)
		return nil, err
	}

	// Copy headers from options to the request
	for key, values := range options.Headers {
		for _, value := range values {
			req.Header.Add(key, value)
		}
	}
	s.logger.Info("Sending external request", "method", options.Method, "url", options.URL)
	res, err := s.httpClient.Do(req)
	if err != nil {
		s.logger.Error("Failed to send request", "error", err, "method", options.Method, "url", options.URL)
		return nil, err
	}

	return res, nil
}

// MakeJSONRequest sends the request, fails on a non-2xx status, decodes the JSON body
// into out (skipped when out is nil) and closes the body. The response headers are
// returned for callers that need them, such as x-total-count.
func (s *Service) MakeJSONRequest(ctx context.Context, options *httpAdapter.HttpOptions, out any) (http.Header, error) {
	res, err := s.MakeRequest(ctx, options)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()

	if res.StatusCode < 200 || res.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(res.Body, 4096))
		s.logger.Error("External request returned an error status", "status", res.StatusCode, "method", options.Method, "url", options.URL, "body", string(body))
		return res.Header, fmt.Errorf("%s %s: unexpected status %d", options.Method, options.URL, res.StatusCode)
	}

	if out != nil {
		if err := json.NewDecoder(res.Body).Decode(out); err != nil {
			s.logger.Error("Failed to decode external response", "error", err, "method", options.Method, "url", options.URL)
			return res.Header, err
		}
	}

	return res.Header, nil
}

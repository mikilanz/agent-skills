package general

import (
	"io"
	"net/http"
)

type HttpOptions struct {
	URL     string
	Method  string
	Headers http.Header
	Body    io.Reader
}

type DefaultHttpResponse struct {
	StatusCode int    `json:"statusCode"`
	Message    string `json:"message"`
	Ok         bool   `json:"ok"`
}

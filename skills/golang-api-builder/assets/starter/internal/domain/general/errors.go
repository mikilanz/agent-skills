package general

import "errors"

// Sentinel errors returned by repositories and services. Handlers map them to HTTP
// status codes with utils.HttpStatusFromError; wrap them with fmt.Errorf("...: %w", ErrX).
var (
	ErrNotFound   = errors.New("data not found")
	ErrDuplicate  = errors.New("duplicate data")
	ErrForbidden  = errors.New("forbidden")
	ErrBadRequest = errors.New("bad request")
)

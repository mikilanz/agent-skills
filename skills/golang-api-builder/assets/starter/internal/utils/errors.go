package utils

import (
	"errors"
	"fmt"
	"net/http"

	"go-api-starter/internal/domain/general"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
)

// TranslateDBError converts pgx errors into domain sentinel errors so services and
// handlers never import pgx. Repositories call it on every error they return.
func TranslateDBError(err error) error {
	if err == nil {
		return nil
	}
	if errors.Is(err, pgx.ErrNoRows) {
		return general.ErrNotFound
	}
	var pgErr *pgconn.PgError
	if errors.As(err, &pgErr) {
		switch pgErr.Code {
		case "23505": // unique_violation
			return fmt.Errorf("%w: %s", general.ErrDuplicate, pgErr.ConstraintName)
		case "23503", "23514", "22P02": // foreign_key_violation, check_violation, invalid_text_representation
			return fmt.Errorf("%w: %s", general.ErrBadRequest, pgErr.Message)
		}
	}
	return err
}

// HttpStatusFromError maps domain sentinel errors to HTTP status codes.
func HttpStatusFromError(err error) int {
	switch {
	case errors.Is(err, general.ErrNotFound):
		return http.StatusNotFound
	case errors.Is(err, general.ErrDuplicate):
		return http.StatusConflict
	case errors.Is(err, general.ErrForbidden):
		return http.StatusForbidden
	case errors.Is(err, general.ErrBadRequest):
		return http.StatusBadRequest
	default:
		return http.StatusInternalServerError
	}
}

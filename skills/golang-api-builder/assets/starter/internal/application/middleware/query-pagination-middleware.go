package middleware

import (
	"log/slog"
	"strconv"
	"strings"

	"go-api-starter/internal/utils"

	"github.com/gin-gonic/gin"
)

const (
	queryRequestKey = "queryRequest"
	defaultPageSize = 10
	maxPageSize     = 1000
)

// QueryPaginationMiddleware populates utils.QueryRequest with filters, pagination, and sort from query params:
//
//	?page=0&size=10&pagination=true&sort=createdAt,desc&name.includes=foo&status.in=a,b
func QueryPaginationMiddleware(logger *slog.Logger) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Parse filters: <field>.<criteria>=<value>
		var filters []utils.QueryFilter
		for key, values := range c.Request.URL.Query() {
			field, filterStr, found := strings.Cut(key, ".")
			if !found || field == "" || strings.Contains(filterStr, ".") {
				continue
			}
			filter := utils.FilterCriteria(filterStr)
			if !isValidFilterCriteria(filter) {
				logger.Warn("Invalid filter criteria in query params", "filter", filterStr, "key", key)
				continue
			}
			valueStr := values[0] // Take first value if multiple

			// 'in'/'notIn' take comma-separated values
			var value interface{}
			if filter == utils.FilterCriteriaIn || filter == utils.FilterCriteriaNotIn {
				parts := strings.Split(valueStr, ",")
				for j, v := range parts {
					parts[j] = strings.TrimSpace(v)
				}
				value = parts
			} else {
				value = valueStr
			}

			filters = append(filters, utils.QueryFilter{
				Key:    field,
				Value:  value,
				Filter: filter,
			})
		}

		// Parse pagination (page is zero-based)
		page := 0
		size := defaultPageSize
		if p := c.Query("page"); p != "" {
			if parsed, err := strconv.Atoi(p); err == nil && parsed >= 0 {
				page = parsed
			}
		}
		if s := c.Query("size"); s != "" {
			if parsed, err := strconv.Atoi(s); err == nil && parsed > 0 {
				size = min(parsed, maxPageSize)
			}
		}
		pagination := c.Query("pagination") == "true"

		// Parse sort: sort=createdAt,desc (repeat the param for multiple sorts)
		var sorts []utils.SortData
		for _, sortParam := range c.QueryArray("sort") {
			parts := strings.Split(sortParam, ",")
			if len(parts) != 2 {
				continue
			}
			field := strings.TrimSpace(parts[0])
			sortOrder := utils.SortOrder(strings.ToLower(strings.TrimSpace(parts[1])))
			if isValidSortOrder(sortOrder) {
				sorts = append(sorts, utils.SortData{
					Key:  field,
					Sort: sortOrder,
				})
			} else {
				logger.Warn("Invalid sort order in query params", "sort", sortParam)
			}
		}

		queryReq := utils.QueryRequest{
			Page:       page,
			Size:       size,
			Pagination: pagination,
			Filters:    filters,
			Sort:       sorts,
		}

		// Store in Gin context for handlers to access
		c.Set(queryRequestKey, queryReq)
		c.Next()
	}
}

// GetQueryRequest returns the QueryRequest QueryPaginationMiddleware stored on the context,
// or the defaults when the middleware didn't run.
func GetQueryRequest(c *gin.Context) utils.QueryRequest {
	if queryRequest, exists := c.Get(queryRequestKey); exists {
		if q, ok := queryRequest.(utils.QueryRequest); ok {
			return q
		}
	}
	return utils.QueryRequest{Page: 0, Size: defaultPageSize}
}

// validate FilterCriteria
func isValidFilterCriteria(f utils.FilterCriteria) bool {
	switch f {
	case utils.FilterCriteriaEqual, utils.FilterCriteriaNotEqual, utils.FilterCriteriaGreaterThanOrEqual,
		utils.FilterCriteriaLessThanOrEqual, utils.FilterCriteriaGreaterThan, utils.FilterCriteriaLessThan,
		utils.FilterCriteriaIncludes, utils.FilterCriteriaIn, utils.FilterCriteriaNotIn,
		utils.FilterCriteriaContains, utils.FilterCriteriaBetween:
		return true
	}
	return false
}

// validate SortOrder
func isValidSortOrder(s utils.SortOrder) bool {
	switch s {
	case utils.SortOrderAsc, utils.SortOrderDesc:
		return true
	}
	return false
}

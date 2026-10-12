package utils

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFilterData(t *testing.T) {
	columns := Columns("t", "name", "stock", "companyCode", "createdAt")

	where, args := FilterData([]QueryFilter{
		{Key: "name", Value: "foo bar", Filter: FilterCriteriaIncludes},
		{Key: "stock", Value: "10", Filter: FilterCriteriaGreaterThanOrEqual},
		{Key: "companyCode", Value: []string{"A", "B"}, Filter: FilterCriteriaIn},
		{Key: "createdAt", Value: "2025-01-01,2025-02-01", Filter: FilterCriteriaBetween},
		{Key: `name" = '' OR 1=1 --`, Value: "x", Filter: FilterCriteriaEqual},
	}, columns)

	assert.Equal(t, ` WHERE (t."name"::text ILIKE ?) AND t."stock" >= ? AND t."companyCode" IN (?, ?) AND (t."createdAt" BETWEEN ? AND ?)`, where)
	assert.Equal(t, []interface{}{"%foo%bar%", "10", "A", "B", "2025-01-01", "2025-02-01"}, args)
}

func TestFilterDataSkipsInvalid(t *testing.T) {
	where, args := FilterData([]QueryFilter{
		{Key: "createdAt", Value: "2025-01-01", Filter: FilterCriteriaBetween},
		{Key: "unknown", Value: "x", Filter: FilterCriteriaEqual},
	}, Columns("", "createdAt"))

	assert.Equal(t, "", where)
	assert.Empty(t, args)
}

func TestBuildSortClause(t *testing.T) {
	columns := Columns("t", "name", "createdAt")

	assert.Equal(t, ` ORDER BY t."createdAt" DESC, t."name" ASC`, BuildSortClause([]SortData{
		{Key: "createdAt", Sort: SortOrderDesc},
		{Key: "name", Sort: SortOrderAsc},
		{Key: "password", Sort: SortOrderAsc},
	}, columns))
	assert.Equal(t, "", BuildSortClause(nil, columns))
}

func TestReplacePlaceholders(t *testing.T) {
	assert.Equal(t, "a = $1 AND b IN ($2, $3) LIMIT $4", ReplacePlaceholders("a = ? AND b IN (?, ?) LIMIT ?"))
}

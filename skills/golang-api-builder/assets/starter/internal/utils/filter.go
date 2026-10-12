package utils

import (
	"fmt"
	"strconv"
	"strings"
)

type FilterCriteria string
type SortOrder string

const (
	FilterCriteriaEqual              FilterCriteria = "equal"
	FilterCriteriaNotEqual           FilterCriteria = "notEqual"
	FilterCriteriaGreaterThanOrEqual FilterCriteria = "greaterThanOrEqual"
	FilterCriteriaLessThanOrEqual    FilterCriteria = "lessThanOrEqual"
	FilterCriteriaGreaterThan        FilterCriteria = "greaterThan"
	FilterCriteriaLessThan           FilterCriteria = "lessThan"
	FilterCriteriaIncludes           FilterCriteria = "includes"
	FilterCriteriaIn                 FilterCriteria = "in"
	FilterCriteriaNotIn              FilterCriteria = "notIn"
	FilterCriteriaContains           FilterCriteria = "contains"
	FilterCriteriaBetween            FilterCriteria = "between"
	SortOrderAsc                     SortOrder      = "asc"
	SortOrderDesc                    SortOrder      = "desc"
)

type QueryFilter struct {
	Key    string
	Value  interface{}
	Filter FilterCriteria
}

type SortData struct {
	Key  string
	Sort SortOrder
}

type QueryRequest struct {
	Page       int
	Size       int
	Pagination bool
	Filters    []QueryFilter
	Sort       []SortData
}

// Columns builds the whitelist FilterData and BuildSortClause accept: API key -> SQL
// column expression. Keys equal the camelCase column names, e.g.
// Columns("p", "name", "createdAt") -> {"name": `p."name"`, "createdAt": `p."createdAt"`}.
// Merge several maps for joins, or write the map by hand to expose a column under another key.
func Columns(alias string, keys ...string) map[string]string {
	columns := make(map[string]string, len(keys))
	for _, key := range keys {
		columns[key] = getAlias(alias) + `"` + key + `"`
	}
	return columns
}

// FilterData builds a WHERE clause with ? placeholders (convert with ReplacePlaceholders).
// Filters whose key is not in columns are skipped: the key comes from the query string,
// so it must never be interpolated into SQL unchecked.
func FilterData(queries []QueryFilter, columns map[string]string) (string, []interface{}) {
	var conditions []string
	var args []interface{}

	for _, query := range queries {
		column, ok := columns[query.Key]
		if !ok {
			continue
		}
		values := filterValues(query.Value)
		valueStr := strings.Join(values, ",")

		switch query.Filter {
		case FilterCriteriaNotEqual:
			conditions = append(conditions, column+" <> ?")
			args = append(args, valueStr)
		case FilterCriteriaGreaterThanOrEqual:
			conditions = append(conditions, column+" >= ?")
			args = append(args, valueStr)
		case FilterCriteriaLessThanOrEqual:
			conditions = append(conditions, column+" <= ?")
			args = append(args, valueStr)
		case FilterCriteriaGreaterThan:
			conditions = append(conditions, column+" > ?")
			args = append(args, valueStr)
		case FilterCriteriaLessThan:
			conditions = append(conditions, column+" < ?")
			args = append(args, valueStr)
		case FilterCriteriaIncludes:
			// every word must appear, in order: "foo bar" -> %foo%bar%
			conditions = append(conditions, "("+column+"::text ILIKE ?)")
			args = append(args, "%"+strings.ReplaceAll(valueStr, " ", "%")+"%")
		case FilterCriteriaContains:
			conditions = append(conditions, column+"::text LIKE ?")
			args = append(args, "%"+valueStr+"%")
		case FilterCriteriaIn, FilterCriteriaNotIn:
			if len(values) == 0 {
				continue
			}
			placeholders := strings.TrimSuffix(strings.Repeat("?, ", len(values)), ", ")
			operator := "IN"
			if query.Filter == FilterCriteriaNotIn {
				operator = "NOT IN"
			}
			conditions = append(conditions, fmt.Sprintf("%s %s (%s)", column, operator, placeholders))
			for _, v := range values {
				args = append(args, v)
			}
		case FilterCriteriaBetween:
			if len(values) < 2 {
				continue
			}
			conditions = append(conditions, "("+column+" BETWEEN ? AND ?)")
			args = append(args, values[0], values[1])
		default: // equal
			conditions = append(conditions, column+" = ?")
			args = append(args, valueStr)
		}
	}

	if len(conditions) == 0 {
		return "", args
	}
	return " WHERE " + strings.Join(conditions, " AND "), args
}

// filterValues normalises a filter value into trimmed strings; comma-separated strings
// are split so "a,b" and []string{"a", "b"} behave the same.
func filterValues(value interface{}) []string {
	var raw []string
	switch v := value.(type) {
	case []string:
		raw = v
	case []interface{}:
		for _, item := range v {
			raw = append(raw, fmt.Sprintf("%v", item))
		}
	case nil:
		return nil
	default:
		raw = strings.Split(fmt.Sprintf("%v", v), ",")
	}

	values := make([]string, 0, len(raw))
	for _, v := range raw {
		values = append(values, strings.TrimSpace(v))
	}
	return values
}

func getAlias(alias string) string {
	if alias != "" {
		return alias + "."
	}
	return ""
}

// ReplacePlaceholders replaces ? in the query string with $1, $2, $3, etc.
// Don't use it on SQL that contains a literal ? (for example the jsonb ? operator).
func ReplacePlaceholders(query string) string {
	var b strings.Builder
	n := 0
	for _, r := range query {
		if r == '?' {
			n++
			b.WriteString("$" + strconv.Itoa(n))
			continue
		}
		b.WriteRune(r)
	}
	return b.String()
}

// BuildSortClause builds an ORDER BY clause from SortData slice; unknown keys are skipped.
func BuildSortClause(sorts []SortData, columns map[string]string) string {
	var clauses []string
	for _, sort := range sorts {
		column, ok := columns[sort.Key]
		if !ok {
			continue
		}
		order := "ASC"
		if sort.Sort == SortOrderDesc {
			order = "DESC"
		}
		clauses = append(clauses, column+" "+order)
	}
	if len(clauses) == 0 {
		return ""
	}
	return " ORDER BY " + strings.Join(clauses, ", ")
}

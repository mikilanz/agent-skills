use chrono::{DateTime, NaiveDate, Utc};
use diesel::pg::Pg;
use diesel::query_builder::AstPass;
use diesel::sql_types::{BigInt, Bool, Double, Integer, Text, Timestamptz};
use diesel::QueryResult;

use crate::delivery::domain::default::QueryRequest;
use crate::delivery::domain::error::QueryError;

/// SQL type of a filterable column; decides how a query-string value is parsed and bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnKind {
    Integer,
    BigInt,
    Text,
    Bool,
    Double,
    Timestamptz,
    Uuid,
}

/// One entry of a repository's filter/sort whitelist. Only listed columns can be filtered or sorted.
#[derive(Clone, Copy, Debug)]
pub struct QueryColumn {
    pub name: &'static str,
    pub kind: ColumnKind,
}

impl QueryColumn {
    pub const fn new(name: &'static str, kind: ColumnKind) -> Self {
        QueryColumn { name, kind }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterValue {
    Integer(i32),
    BigInt(i64),
    Text(String),
    Bool(bool),
    Double(f64),
    Timestamptz(DateTime<Utc>),
    Uuid(uuid::Uuid),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Criteria {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Includes,
    Contains,
    In,
    NotIn,
    Between,
}

impl Criteria {
    pub fn parse(value: &str) -> Result<Self, QueryError> {
        match value {
            "equal" => Ok(Criteria::Equal),
            "notEqual" => Ok(Criteria::NotEqual),
            // "greather*" is a legacy spelling some clients still send
            "greaterThan" | "greatherThan" => Ok(Criteria::GreaterThan),
            "greaterThanOrEqual" | "greatherThanOrEqual" => Ok(Criteria::GreaterThanOrEqual),
            "lessThan" => Ok(Criteria::LessThan),
            "lessThanOrEqual" => Ok(Criteria::LessThanOrEqual),
            "includes" => Ok(Criteria::Includes),
            "contains" => Ok(Criteria::Contains),
            "in" => Ok(Criteria::In),
            "notIn" => Ok(Criteria::NotIn),
            "between" => Ok(Criteria::Between),
            other => Err(QueryError::UnknownCriteria(other.to_owned())),
        }
    }

    fn operator(&self) -> &'static str {
        match self {
            Criteria::NotEqual => " <> ",
            Criteria::GreaterThan => " > ",
            Criteria::GreaterThanOrEqual => " >= ",
            Criteria::LessThan => " < ",
            Criteria::LessThanOrEqual => " <= ",
            _ => " = ",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedFilter {
    pub column: &'static str,
    pub criteria: Criteria,
    pub values: Vec<FilterValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedSort {
    pub column: &'static str,
    pub descending: bool,
}

fn find_column(columns: &'static [QueryColumn], key: &str) -> Result<&'static QueryColumn, QueryError> {
    columns
        .iter()
        .find(|column| column.name == key)
        .ok_or_else(|| QueryError::UnknownField(key.to_owned()))
}

fn parse_value(column: &QueryColumn, raw: &str) -> Result<FilterValue, QueryError> {
    let invalid = || QueryError::InvalidValue {
        field: column.name.to_owned(),
        value: raw.to_owned(),
    };
    let trimmed = raw.trim();

    match column.kind {
        ColumnKind::Integer => trimmed.parse::<i32>().map(FilterValue::Integer).map_err(|_| invalid()),
        ColumnKind::BigInt => trimmed.parse::<i64>().map(FilterValue::BigInt).map_err(|_| invalid()),
        ColumnKind::Double => trimmed.parse::<f64>().map(FilterValue::Double).map_err(|_| invalid()),
        ColumnKind::Text => Ok(FilterValue::Text(raw.to_owned())),
        ColumnKind::Bool => match trimmed {
            "true" | "1" => Ok(FilterValue::Bool(true)),
            "false" | "0" => Ok(FilterValue::Bool(false)),
            _ => Err(invalid()),
        },
        ColumnKind::Timestamptz => DateTime::parse_from_rfc3339(trimmed)
            .map(|date| date.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                    .map(|date| date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc())
            })
            .map(FilterValue::Timestamptz)
            .map_err(|_| invalid()),
        ColumnKind::Uuid => uuid::Uuid::parse_str(trimmed)
            .map(FilterValue::Uuid)
            .map_err(|_| invalid()),
    }
}

fn escape_like(value: &str) -> String {
    value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Validates every filter of the request against `columns` and parses its values into typed binds.
pub fn parse_filters(query: &QueryRequest, columns: &'static [QueryColumn]) -> Result<Vec<ParsedFilter>, QueryError> {
    query
        .filter
        .iter()
        .map(|filter| {
            let column = find_column(columns, &filter.key)?;
            let criteria = Criteria::parse(&filter.filter)?;

            let values = match criteria {
                // case-sensitive substring
                Criteria::Includes => vec![FilterValue::Text(format!("%{}%", escape_like(&filter.value)))],
                // substring where spaces match anything: "foo bar" -> %foo%bar%
                Criteria::Contains => vec![FilterValue::Text(format!(
                    "%{}%",
                    escape_like(&filter.value).replace(' ', "%")
                ))],
                Criteria::In | Criteria::NotIn => filter
                    .value
                    .split(',')
                    .map(|value| parse_value(column, value))
                    .collect::<Result<Vec<_>, _>>()?,
                Criteria::Between => {
                    let bounds = filter.value.split(',').collect::<Vec<&str>>();
                    if bounds.len() != 2 {
                        return Err(QueryError::InvalidValue {
                            field: column.name.to_owned(),
                            value: filter.value.clone(),
                        });
                    }
                    vec![parse_value(column, bounds[0])?, parse_value(column, bounds[1])?]
                }
                _ => vec![parse_value(column, &filter.value)?],
            };

            Ok(ParsedFilter {
                column: column.name,
                criteria,
                values,
            })
        })
        .collect()
}

/// Parses `sort=<column>,ASC|DESC`. An empty sort means no ORDER BY.
pub fn parse_sort(sort: &str, columns: &'static [QueryColumn]) -> Result<Option<ParsedSort>, QueryError> {
    if sort.trim().is_empty() {
        return Ok(None);
    }

    let (key, direction) = sort.split_once(',').unwrap_or((sort, "ASC"));
    let column = find_column(columns, key.trim())?;

    let descending = match direction.trim().to_uppercase().as_str() {
        "ASC" => false,
        "DESC" => true,
        _ => {
            return Err(QueryError::InvalidValue {
                field: "sort".to_owned(),
                value: sort.to_owned(),
            })
        }
    };

    Ok(Some(ParsedSort {
        column: column.name,
        descending,
    }))
}

fn push_column(column: &str, out: &mut AstPass<'_, '_, Pg>) -> QueryResult<()> {
    out.push_sql("t.");
    out.push_identifier(column)
}

fn push_value<'b>(value: &'b FilterValue, out: &mut AstPass<'_, 'b, Pg>) -> QueryResult<()> {
    match value {
        FilterValue::Integer(v) => out.push_bind_param::<Integer, _>(v),
        FilterValue::BigInt(v) => out.push_bind_param::<BigInt, _>(v),
        FilterValue::Text(v) => out.push_bind_param::<Text, _>(v),
        FilterValue::Bool(v) => out.push_bind_param::<Bool, _>(v),
        FilterValue::Double(v) => out.push_bind_param::<Double, _>(v),
        FilterValue::Timestamptz(v) => out.push_bind_param::<Timestamptz, _>(v),
        FilterValue::Uuid(v) => out.push_bind_param::<diesel::sql_types::Uuid, _>(v),
    }
}

/// Writes ` WHERE t."col" <op> $n AND ...` for the subquery aliased `t`. Column names come from the
/// whitelist and every value is a bind parameter, so nothing from the client is spliced into SQL.
pub fn push_filters<'b>(filters: &'b [ParsedFilter], out: &mut AstPass<'_, 'b, Pg>) -> QueryResult<()> {
    for (index, filter) in filters.iter().enumerate() {
        out.push_sql(if index == 0 { " WHERE " } else { " AND " });

        match filter.criteria {
            Criteria::Includes | Criteria::Contains => {
                out.push_sql("CAST(");
                push_column(filter.column, out)?;
                out.push_sql(" AS TEXT) LIKE ");
                push_value(&filter.values[0], out)?;
            }
            Criteria::In | Criteria::NotIn => {
                push_column(filter.column, out)?;
                out.push_sql(if filter.criteria == Criteria::In {
                    " IN ("
                } else {
                    " NOT IN ("
                });
                for (value_index, value) in filter.values.iter().enumerate() {
                    if value_index > 0 {
                        out.push_sql(", ");
                    }
                    push_value(value, out)?;
                }
                out.push_sql(")");
            }
            Criteria::Between => {
                push_column(filter.column, out)?;
                out.push_sql(" BETWEEN ");
                push_value(&filter.values[0], out)?;
                out.push_sql(" AND ");
                push_value(&filter.values[1], out)?;
            }
            _ => {
                push_column(filter.column, out)?;
                out.push_sql(filter.criteria.operator());
                push_value(&filter.values[0], out)?;
            }
        }
    }

    Ok(())
}

pub fn push_sort(sort: &Option<ParsedSort>, out: &mut AstPass<'_, '_, Pg>) -> QueryResult<()> {
    if let Some(sort) = sort {
        out.push_sql(" ORDER BY ");
        push_column(sort.column, out)?;
        out.push_sql(if sort.descending { " DESC" } else { " ASC" });
    }
    Ok(())
}

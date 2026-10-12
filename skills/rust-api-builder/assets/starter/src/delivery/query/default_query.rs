use serde::Deserialize;
use utoipa::IntoParams;

/// Documents the list query-string contract in Swagger (`params(DefaultQuery)`). The values are
/// parsed by `utils::convert::query_transform`; filters are `<column>.<criteria>=<value>`, e.g.
/// `name.includes=abc&stock.greaterThan=10&status.in=active,inactive`.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[allow(dead_code)]
pub struct DefaultQuery {
    /// zero-based page, default 0
    page: Option<i64>,
    /// page size, default 10, max 1000
    size: Option<i64>,
    /// `<column>,ASC|DESC`, default `id,DESC`
    sort: Option<String>,
}

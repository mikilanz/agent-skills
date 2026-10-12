use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::BigInt;

use crate::delivery::domain::default::QueryRequest;
use crate::delivery::domain::error::QueryError;
use crate::utils::filter::{parse_filters, parse_sort, push_filters, push_sort, ParsedFilter, ParsedSort, QueryColumn};

pub const DEFAULT_PER_PAGE: i64 = 10;
pub const MAX_PER_PAGE: i64 = 1000;

pub trait Paginate: Sized {
    /// Wraps the query as `SELECT *, COUNT(*) OVER () FROM (<query>) t WHERE … ORDER BY … LIMIT … OFFSET …`
    /// using the request's page, size, sort and filters, validated against `columns`.
    fn paginate(self, query: &QueryRequest, columns: &'static [QueryColumn]) -> Result<Paginated<Self>, QueryError>;
}

impl<T> Paginate for T {
    fn paginate(self, query: &QueryRequest, columns: &'static [QueryColumn]) -> Result<Paginated<Self>, QueryError> {
        let page = query.page.max(0);
        let per_page = query.size.clamp(1, MAX_PER_PAGE);

        Ok(Paginated {
            query: self,
            page,
            per_page,
            offset: page * per_page,
            filters: parse_filters(query, columns)?,
            sort: parse_sort(&query.sort, columns)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
    offset: i64,
    filters: Vec<ParsedFilter>,
    sort: Option<ParsedSort>,
}

// The SQL text depends on the filters, so the query has no static id.
impl<T> QueryId for Paginated<T> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T> Paginated<T> {
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated {
            per_page,
            offset: self.page * per_page,
            ..self
        }
    }

    /// Returns the page and the total number of rows matching the filters.
    pub fn load_and_count_pages<'a, U>(self, conn: &mut PgConnection) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<'a, PgConnection, (U, i64)>,
    {
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.first().map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();

        Ok((records, total))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t");
        push_filters(&self.filters, &mut out)?;
        push_sort(&self.sort, &mut out)?;
        out.push_sql(" LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::convert::query_transform;
    use crate::utils::filter::ColumnKind;

    diesel::table! {
        items (id) {
            id -> Int4,
            name -> Varchar,
            stock -> Int4,
            created_at -> Timestamptz,
        }
    }

    const ITEM_COLUMNS: &[QueryColumn] = &[
        QueryColumn::new("id", ColumnKind::Integer),
        QueryColumn::new("name", ColumnKind::Text),
        QueryColumn::new("stock", ColumnKind::Integer),
        QueryColumn::new("created_at", ColumnKind::Timestamptz),
    ];

    fn sql_for(query_string: &str) -> Result<String, QueryError> {
        let query = query_transform(query_string)?;
        let paginated = items::table.into_boxed::<Pg>().paginate(&query, ITEM_COLUMNS)?;
        Ok(diesel::debug_query::<Pg, _>(&paginated).to_string())
    }

    #[test]
    fn binds_filters_sort_and_paging() {
        let sql = sql_for("page=2&size=5&sort=name,ASC&name.includes=a%25b&stock.greaterThan=10&id.in=1,2,3").unwrap();

        assert!(sql.starts_with("SELECT *, COUNT(*) OVER () FROM (SELECT"), "{}", sql);
        assert!(
            sql.contains(r#") t WHERE CAST(t."name" AS TEXT) LIKE $1 AND t."stock" > $2 AND t."id" IN ($3, $4, $5) ORDER BY t."name" ASC LIMIT $6 OFFSET $7"#),
            "{}",
            sql
        );
        assert!(sql.contains(r#"binds: ["%a\\%b%", 10, 1, 2, 3, 5, 10]"#), "{}", sql);
    }

    #[test]
    fn default_sort_is_id_desc() {
        let sql = sql_for("").unwrap();
        assert!(sql.contains(r#"ORDER BY t."id" DESC LIMIT $1 OFFSET $2"#), "{}", sql);
    }

    #[test]
    fn rejects_unknown_columns_criteria_and_bad_values() {
        assert_eq!(
            sql_for("password.equal=x"),
            Err(QueryError::UnknownField("password".into()))
        );
        assert_eq!(
            sql_for("sort=password,DESC"),
            Err(QueryError::UnknownField("password".into()))
        );
        assert_eq!(sql_for("name.like=x"), Err(QueryError::UnknownCriteria("like".into())));
        assert!(matches!(
            sql_for("stock.equal=abc"),
            Err(QueryError::InvalidValue { .. })
        ));
        assert!(matches!(
            sql_for("stock.between=1"),
            Err(QueryError::InvalidValue { .. })
        ));
        assert!(matches!(
            sql_for("sort=name,sideways"),
            Err(QueryError::InvalidValue { .. })
        ));
    }

    #[test]
    fn injection_attempts_stay_in_binds() {
        let sql = sql_for("name.equal=x'%20OR%201=1--").unwrap();
        assert!(sql.contains(r#"WHERE t."name" = $1"#), "{}", sql);
        assert!(sql.contains(r#""x' OR 1=1--""#), "{}", sql);
    }

    #[test]
    fn caps_page_size() {
        let sql = sql_for("size=100000").unwrap();
        assert!(sql.contains("binds: [1000, 0]"), "{}", sql);
    }
}

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse};
use diesel::{sql_query, RunQueryDsl};

use crate::application::modules::general_modules::GeneralModules;
use crate::delivery::domain::default::DefaultHttpResponseDTO;
use crate::delivery::domain::error::{CustomErrorResponse, ErrorPresenter};
use crate::usecases::redis_service::RedisService;
use crate::utils::db::run_blocking;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health_check);
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Succesfully", body = DefaultHttpResponseDTO),
        (status = 503, description = "Database or redis unreachable", body = ErrorPresenter)
    )
)]
#[get("")]
async fn health_check(ctx: web::Data<GeneralModules>) -> Result<HttpResponse, CustomErrorResponse> {
    run_blocking(&ctx.pool, |conn| {
        sql_query("SELECT 1").execute(conn)?;
        Ok(())
    })
    .await
    .map_err(|e| {
        log::error!("health check database: {}", e);
        CustomErrorResponse::new(StatusCode::SERVICE_UNAVAILABLE, "database unreachable")
    })?;

    RedisService::new(&ctx.redis).ping().await.map_err(|e| {
        log::error!("health check redis: {}", e);
        CustomErrorResponse::new(StatusCode::SERVICE_UNAVAILABLE, "redis unreachable")
    })?;

    Ok(HttpResponse::Ok().json(DefaultHttpResponseDTO::success("ok")))
}

pub mod application;
pub mod config;
pub mod dataproviders;
pub mod delivery;
pub mod migrations;
pub mod usecases;
pub mod utils;

use std::net::TcpListener;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use tokio_cron_scheduler::JobScheduler;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::application::modules::general_modules::GeneralModules;
use crate::config::config_service::ConfigService;
use crate::delivery::domain::default::DefaultHttpResponseDTO;
use crate::delivery::domain::error::ErrorPresenter;
use crate::delivery::rest::health_controller;
use crate::migrations::migration_table::run_migrations;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme("authorization", SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)))
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Rust API Starter", description = "Rust API Starter endpoints"),
    paths(
        health_controller::health_check,
    ),
    components(
        schemas(
            DefaultHttpResponseDTO,
            ErrorPresenter,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

pub async fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let config_service = ConfigService {};

    // one pool for the whole process; repositories hold clones of it
    let pool = config_service.get_pool_connection_db();

    run_migrations(&pool).map_err(|e| std::io::Error::other(format!("Error applying migrations: {}", e)))?;
    log::info!("Migrations applied successfully!");

    let redis = config_service.get_redis_connection().await;

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(std::io::Error::other)?;

    let scheduler = JobScheduler::new().await.map_err(std::io::Error::other)?;
    scheduler.start().await.map_err(std::io::Error::other)?;

    let data = web::Data::new(GeneralModules {
        app_name: String::from("Rust API Starter"),
        pool: pool.clone(),
        config_service: config_service.clone(),
        redis,
        http_client,
        scheduler,
    });

    let openapi = ApiDoc::openapi();
    let port = listener.local_addr()?.port();

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(["x-total-count"])
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(data.clone())
            .service(web::scope("/health").configure(health_controller::routes))
            // .service(web::scope("/api/v1/<feature>").configure(<feature>_controller::routes))
            .service(SwaggerUi::new("/api/{_:.*}").url("/api/openapi.json", openapi.clone()))
    })
    .listen(listener)?
    .run();

    log::info!("Server running on port {}", port);

    Ok(server)
}

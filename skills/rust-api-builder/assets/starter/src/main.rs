use std::env;
use std::net::TcpListener;

use rust_api_starter::config::config_service::ConfigService;
use rust_api_starter::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ENV=staging loads .env.staging, otherwise .env
    let environment_file = match env::var("ENV") {
        Ok(e) => format!(".env.{}", e),
        Err(_) => String::from(".env"),
    };
    dotenvy::from_filename(environment_file).ok();

    // stdout, so the platform's log collector (Cloud Logging, kubectl logs) picks it up
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stdout)
        .init();

    let port = ConfigService {}.get_env("APP_PORT");
    let listener =
        TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap_or_else(|_| panic!("Failed to bind port {}", port));

    run(listener).await?.await
}

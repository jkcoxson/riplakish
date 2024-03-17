// Jackson Coxson

use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};

use axum_client_ip::InsecureClientIp;
use log::{info, warn};
use rand::Rng;
use statics::*;
use tower_http::cors::CorsLayer;

mod db;
mod statics;

#[tokio::main]
async fn main() {
    println!("Starting server");
    dotenv::dotenv().ok();
    env_logger::init();
    info!("Logger initialized");

    let database = db::Database::new();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any);

    // build our application with a single route
    let app = Router::new()
        .route("/admin", get(html))
        .route("/scripts.js", get(js))
        .route("/styles.css", get(css))
        .route("/r/:code", get(redirect))
        .route("/base", get(base_url))
        .route("/admin/stats", get(get_stats))
        .route("/admin/logs/:code", get(get_logs))
        .route("/admin/add/*url", post(add_url))
        .route("/admin/remove/*url", delete(remove_url))
        .route("/admin/modify/:code/*new_url", post(modify_url))
        .fallback(fallback)
        .with_state(database)
        .layer(cors);

    let port = std::env::var("RIPLAKISH_PORT").unwrap_or("3009".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn redirect(
    Path(code): Path<String>,
    State(database): State<db::Database>,
    headers: HeaderMap,
    insecure_ip: InsecureClientIp,
) -> Result<axum::response::Redirect, (StatusCode, &'static str)> {
    if let Some(redirect) = database.get_url(code.clone()).await {
        let ip = if database.behind_traefik {
            if let Some(h) = headers.get("X-Forwarded-For") {
                h.to_str().unwrap_or("unknown").to_string()
            } else {
                "unknown".to_string()
            }
        } else {
            insecure_ip.0.to_string()
        };
        database.log(code, redirect.clone(), ip).await;
        return Ok(axum::response::Redirect::to(redirect.as_str()));
    }
    Err((StatusCode::NOT_FOUND, "404 Not Found\n-- Riplakish --"))
}

async fn base_url(State(database): State<db::Database>) -> String {
    database.base_url
}

async fn get_stats(State(database): State<db::Database>) -> Json<Vec<db::DatabaseStats>> {
    axum::Json(database.get_stats().await)
}

async fn get_logs(
    State(database): State<db::Database>,
    Path(code): Path<String>,
) -> Json<Vec<db::DatabaseLog>> {
    info!("Getting the logs for {code}");
    axum::Json(database.get_logs(code).await)
}

async fn add_url(
    Path(url): Path<String>,
    State(database): State<db::Database>,
) -> Result<(StatusCode, String), StatusCode> {
    let s: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();
    info!("Attempting to insert {url} with code {s}");
    if database.insert_url(&url, &s).await {
        Ok((StatusCode::OK, s))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn remove_url(Path(code): Path<String>, State(database): State<db::Database>) -> StatusCode {
    warn!("Removing redirect code {code}");
    if database.remove_url(code).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn modify_url(
    Path((code, new_url)): Path<(String, String)>,
    State(database): State<db::Database>,
) -> StatusCode {
    info!("Updating {code} to new URL: {new_url}");
    if database.modify_url(code, new_url).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found\n-- Riplakish --")
}

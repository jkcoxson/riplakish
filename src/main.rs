// Jackson Coxson

use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    http::{
        header::{CONTENT_TYPE, SET_COOKIE},
        HeaderMap, HeaderName, Method, StatusCode,
    },
    response::Response,
    routing::{delete, get, post},
    Router,
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
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([
            CONTENT_TYPE,
            HeaderName::from_static("x-password"),
            HeaderName::from_static("x-username"),
            HeaderName::from_static("x-token"),
        ]);

    // build our application with a single route
    let app = Router::new()
        .route("/admin", get(html))
        .route("/admin/login", get(login))
        .route("/scripts.js", get(js))
        .route("/styles.css", get(css))
        .route("/r/:code", get(redirect))
        .route("/base", get(base_url))
        .route("/admin/stats", get(get_stats))
        .route("/admin/logs/:code", get(get_logs))
        .route("/admin/add/*url", post(add_url))
        .route("/admin/remove/*url", delete(remove_url))
        .route("/admin/modify/:code/*new_url", post(modify_url))
        .route(
            "/admin/modify-comment/:code/*new_comment",
            post(modify_comment),
        )
        .fallback(fallback)
        .layer(cors)
        .with_state(database);

    let port = std::env::var("RIPLAKISH_PORT").unwrap_or("3009".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
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
    if let Some(redirect) = database.get_url(code.clone()) {
        let ip = if database.behind_traefik {
            if let Some(h) = headers.get("X-Forwarded-For") {
                h.to_str().unwrap_or("unknown").to_string()
            } else {
                "unknown".to_string()
            }
        } else {
            insecure_ip.0.to_string()
        };
        database.log(code, redirect.clone(), ip);
        return Ok(axum::response::Redirect::to(redirect.as_str()));
    }
    Err((StatusCode::NOT_FOUND, "404 Not Found\n-- Riplakish --"))
}

async fn login(State(database): State<db::Database>, headers: HeaderMap) -> Response {
    // Get the username and password
    let username = headers.get("X-Username").and_then(|h| h.to_str().ok());
    let password = headers.get("X-Password").and_then(|h| h.to_str().ok());

    if username.is_none() || password.is_none() {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Default::default())
            .unwrap();
    }

    if username.unwrap() != database.username || password.unwrap() != database.password {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Default::default())
            .unwrap();
    }

    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    database.insert_token(token.clone());

    Response::builder()
        .status(StatusCode::OK)
        .header(
            SET_COOKIE,
            format!("X-Token={token}; SameSite=Strict; HttpOnly"),
        )
        .body(Default::default())
        .unwrap()
}

#[inline]
async fn check_login(database: &db::Database, headers: &HeaderMap) -> bool {
    let cookies = headers.get("cookie").and_then(|h| h.to_str().ok());
    if let Some(cookies) = cookies {
        let cookies = cookies.split(';').collect::<Vec<&str>>();
        for cookie in cookies {
            if cookie.starts_with("X-Token=") {
                let token = cookie.split('=').collect::<Vec<&str>>()[1];
                let token = token.replace(" SameSite", "");
                return database.check_token(token.trim().to_string());
            }
        }
    }
    false
}

async fn base_url(State(database): State<db::Database>) -> String {
    database.base_url
}

async fn get_stats(State(database): State<db::Database>, headers: HeaderMap) -> Response {
    info!("Getting the stats...");

    if check_login(&database, &headers).await {
        Response::builder()
            .status(StatusCode::OK)
            .body(serde_json::to_string(&database.get_stats()).unwrap().into())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Default::default())
            .unwrap()
    }
}

async fn get_logs(
    State(database): State<db::Database>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Response {
    info!("Getting the logs for {code}");

    if check_login(&database, &headers).await {
        Response::builder()
            .status(StatusCode::OK)
            .body(
                serde_json::to_string(&database.get_logs(code))
                    .unwrap()
                    .into(),
            )
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Default::default())
            .unwrap()
    }
}

async fn add_url(
    Path(url): Path<String>,
    State(database): State<db::Database>,
    headers: HeaderMap,
) -> Result<(StatusCode, String), StatusCode> {
    if check_login(&database, &headers).await {
        let s: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(4)
            .map(char::from)
            .collect();
        info!("Attempting to insert {url} with code {s}");
        if database.insert_url(&url, &s) {
            Ok((StatusCode::OK, s))
        } else {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn remove_url(
    Path(code): Path<String>,
    State(database): State<db::Database>,
    headers: HeaderMap,
) -> StatusCode {
    warn!("Removing redirect code {code}");

    if check_login(&database, &headers).await {
        if database.remove_url(code) {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        }
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn modify_url(
    Path((code, new_url)): Path<(String, String)>,
    State(database): State<db::Database>,
    headers: HeaderMap,
) -> StatusCode {
    info!("Updating {code} to new URL: {new_url}");

    if check_login(&database, &headers).await {
        if database.modify_url(code, new_url) {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        }
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn modify_comment(
    Path((code, new_comment)): Path<(String, String)>,
    State(database): State<db::Database>,
    headers: HeaderMap,
) -> StatusCode {
    info!("Updating {code} to new comment: {new_comment}");

    if check_login(&database, &headers).await {
        if database.modify_comment(code, new_comment) {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        }
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found\n-- Riplakish --")
}

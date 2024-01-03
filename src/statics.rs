// Jackson Coxson

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

pub async fn html() -> Html<&'static str> {
    Html(include_str!("../frontend/dist/index.html"))
}

pub async fn js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/javascript")],
        include_str!("../frontend/dist/scripts.js"),
    )
}

pub async fn css() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/css")],
        include_str!("../frontend/dist/styles.css"),
    )
}

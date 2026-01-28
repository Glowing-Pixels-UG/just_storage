use askama::Template;
use axum::{
    response::{Html, IntoResponse, Response},
    http::StatusCode,
};

#[derive(Template)]
#[template(path = "internal/health.html")]
pub struct HealthTemplate {
    pub service_name: String,
    pub version: String,
    pub db_status: String,
    pub db_latency: String,
    pub hot_storage_usage: String,
    pub cold_storage_usage: String,
    pub total_objects: i64,
}

impl IntoResponse for HealthTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template rendering error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                )
                    .into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "internal/base.html")]
pub struct BaseTemplate<T: std::fmt::Display> {
    pub title: String,
    pub content: T,
}

impl<T: std::fmt::Display> IntoResponse for BaseTemplate<T> {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template rendering error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                )
                    .into_response()
            }
        }
    }
}

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "internal/health.html")]
pub struct HealthTemplate {
    pub csrf_token: String,
    pub service_name: String,
    pub version: String,
    pub uptime: String,
    pub db_status: String,
    pub db_latency: String,
    pub db_pool_active: u32,
    pub db_pool_idle: u32,
    pub db_pool_max: u32,
    pub hot_storage_usage: String,
    pub cold_storage_usage: String,
    pub hot_storage_path: String,
    pub cold_storage_path: String,
    pub total_objects: i64,
    pub gc_status: String,
    pub gc_last_run: String,
    pub gc_next_run: String,
    pub gc_total_deleted: usize,
}

impl IntoResponse for HealthTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template rendering error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "internal/base.html")]
pub struct BaseTemplate<T: std::fmt::Display> {
    pub title: String,
    pub content: T,
    pub csrf_token: String,
}

impl<T: std::fmt::Display> IntoResponse for BaseTemplate<T> {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template rendering error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "internal/login.html")]
pub struct LoginTemplate {
    pub title: String,
    pub error: Option<String>,
    pub csrf_token: String,
}

impl IntoResponse for LoginTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("Template rendering error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

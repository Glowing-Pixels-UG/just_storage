#[cfg(test)]
mod tests {
    use crate::api::handlers::health_handler;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_health_handler() {
        let (status, body) = health_handler().await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.0["status"], "healthy");
    }
}

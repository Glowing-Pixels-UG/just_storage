use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use tower_cookies::{Cookie, Cookies};

use crate::api::internal::templates::LoginTemplate;
use crate::api::router::AppState;

#[derive(Deserialize)]
pub struct LoginPayload {
    token: String,
}

pub async fn login_page() -> impl IntoResponse {
    LoginTemplate {
        title: "Admin Login".to_string(),
        error: None,
    }
}

pub async fn login_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(payload): Form<LoginPayload>,
) -> impl IntoResponse {
    let expected_token = match &state.config.admin_token {
        Some(token) => token,
        None => {
            return LoginTemplate {
                title: "Admin Login".to_string(),
                error: Some("Admin access is disabled (token not configured)".to_string()),
            }
            .into_response()
        }
    };

    if payload.token == *expected_token {
        let mut cookie = Cookie::new("admin_session", payload.token);
        cookie.set_path("/dashboard");
        cookie.set_http_only(true);
        cookie.set_secure(true); // Should be true in prod
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
        
        cookies.add(cookie);
        
        Redirect::to("/dashboard/health").into_response()
    } else {
        LoginTemplate {
            title: "Admin Login".to_string(),
            error: Some("Invalid admin token".to_string()),
        }
        .into_response()
    }
}

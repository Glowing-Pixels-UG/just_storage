use axum::http::Method;
use serde_json::json;

mod common;
use common::http;

#[test]
fn test_http_helpers_request_builders() {
    let get = http::get_request("/test");
    assert_eq!(get.method(), Method::GET);
    assert_eq!(get.uri().path(), "/test");

    let post = http::post_request("/p", json!({"a":1}));
    assert_eq!(post.method(), Method::POST);
    assert_eq!(
        post.headers().get("content-type").unwrap(),
        "application/json"
    );

    let auth = http::authenticated_request(Method::PUT, "/x", "key");
    assert_eq!(auth.headers().get("authorization").unwrap(), "Bearer key");

    let auth_json = http::authenticated_json_request(Method::POST, "/y", "key", json!({"x":2}));
    assert_eq!(
        auth_json.headers().get("authorization").unwrap(),
        "Bearer key"
    );
    assert_eq!(
        auth_json.headers().get("content-type").unwrap(),
        "application/json"
    );
}

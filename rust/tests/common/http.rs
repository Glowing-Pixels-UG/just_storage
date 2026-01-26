#![allow(dead_code)]

//! HTTP helpers for tests (request builders)

use axum::body::Body;
use axum::http::Method;
use axum::http::Request;
use serde_json::Value;

/// Create a JSON request body
pub fn json_body(data: Value) -> Body {
    Body::from(serde_json::to_string(&data).unwrap())
}

/// Create a GET request
pub fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Create a POST request with JSON body
pub fn post_request(uri: &str, data: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json")
        .body(json_body(data))
        .unwrap()
}

/// Create a PUT request with JSON body
pub fn put_request(uri: &str, data: Value) -> Request<Body> {
    Request::builder()
        .method(Method::PUT)
        .uri(uri)
        .header("content-type", "application/json")
        .body(json_body(data))
        .unwrap()
}

/// Create a DELETE request
pub fn delete_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Create an authenticated request
pub fn authenticated_request(method: Method, uri: &str, api_key: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", api_key))
        .body(Body::empty())
        .unwrap()
}

/// Create an authenticated request with JSON body
pub fn authenticated_json_request(
    method: Method,
    uri: &str,
    api_key: &str,
    data: Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .body(json_body(data))
        .unwrap()
}

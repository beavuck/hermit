use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{MatchedPath, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::MethodRouter,
};

use crate::http_method::HttpMethod;
use crate::spec::RouteConfig;

#[derive(Clone)]
pub struct AppState {
    routes: HashMap<String, RouteConfig>,
}

pub fn build(configs: Vec<RouteConfig>) -> Router {
    let mut by_path: HashMap<String, MethodRouter<Arc<AppState>>> = HashMap::new();

    for cfg in &configs {
        let mr = match cfg.method {
            HttpMethod::Get => axum::routing::get(handle_readonly),
            HttpMethod::Delete => axum::routing::delete(handle_readonly),
            HttpMethod::Options => axum::routing::options(handle_readonly),
            HttpMethod::Head => axum::routing::head(handle_readonly),
            HttpMethod::Trace => axum::routing::trace(handle_readonly),
            HttpMethod::Post => axum::routing::post(handle_with_body),
            HttpMethod::Put => axum::routing::put(handle_with_body),
            HttpMethod::Patch => axum::routing::patch(handle_with_body),
        };

        let path = cfg.axum_path.clone();
        by_path
            .entry(path)
            .and_modify(|existing| {
                let combined = std::mem::replace(existing, MethodRouter::new()).merge(mr.clone());
                *existing = combined;
            })
            .or_insert(mr);
    }

    let state = Arc::new(AppState {
        routes: into_state_map(configs),
    });

    let mut router = Router::new();
    for (path, mr) in by_path {
        router = router.route(&path, mr);
    }
    router.with_state(state)
}

fn into_state_map(configs: Vec<RouteConfig>) -> HashMap<String, RouteConfig> {
    configs
        .into_iter()
        .map(|cfg| {
            let key = route_key(&cfg.method.as_str().to_uppercase(), &cfg.axum_path);
            (key, cfg)
        })
        .collect()
}

fn route_key(method: &str, path: &str) -> String {
    format!("{} {}", method, path)
}

async fn handle_readonly(
    State(state): State<Arc<AppState>>,
    matched: MatchedPath,
    method: Method,
) -> Response {
    let key = route_key(method.as_str(), matched.as_str());
    match state.routes.get(&key) {
        Some(cfg) if cfg.status_code == StatusCode::NO_CONTENT.as_u16() => {
            StatusCode::NO_CONTENT.into_response()
        }
        Some(cfg) => json_response(cfg.status_code, cfg.body.clone()),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_with_body(
    State(state): State<Arc<AppState>>,
    matched: MatchedPath,
    method: Method,
    body: Option<Json<serde_json::Value>>,
) -> Response {
    let key = route_key(method.as_str(), matched.as_str());
    let cfg = match state.routes.get(&key) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let request_fields = body.map(|Json(b)| b);

    let base = if let (Some(disc_field), Some(variants)) = (&cfg.discriminator_field, &cfg.variants)
    {
        let disc_value = request_fields
            .as_ref()
            .and_then(|b| b.get(disc_field))
            .and_then(|v| v.as_str());

        disc_value
            .and_then(|d| variants.get(d))
            .or_else(|| variants.values().next())
            .cloned()
    } else {
        cfg.body.clone()
    };

    json_response(cfg.status_code, Some(merge(base, request_fields)))
}

fn merge(base: Option<serde_json::Value>, overlay: Option<serde_json::Value>) -> serde_json::Value {
    let mut result = base.unwrap_or(serde_json::Value::Null);
    if let (Some(obj), Some(serde_json::Value::Object(fields))) = (result.as_object_mut(), overlay)
    {
        for (k, v) in fields {
            obj.insert(k, v);
        }
    }
    result
}

fn json_response(status_code: u16, body: Option<serde_json::Value>) -> Response {
    let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
    let payload = body.unwrap_or(serde_json::Value::Null);
    (status, Json(payload)).into_response()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::merge;

    // --- merge ---

    #[test]
    fn overlay_value_wins_over_base() {
        let base = json!({ "storyPoints": 5, "type": "feature" });
        let overlay = json!({ "storyPoints": 6 });
        let result = merge(Some(base), Some(overlay));
        assert_eq!(result["storyPoints"], json!(6));
    }

    #[test]
    fn base_value_kept_when_not_in_overlay() {
        let base = json!({ "id": "abc", "type": "feature" });
        let overlay = json!({ "type": "feature" });
        let result = merge(Some(base), Some(overlay));
        assert_eq!(result["id"], json!("abc"));
    }

    #[test]
    fn unknown_overlay_field_is_included() {
        let base = json!({ "type": "feature" });
        let overlay = json!({ "type": "feature", "coreRepo": "IAM" });
        let result = merge(Some(base), Some(overlay));
        assert_eq!(result["coreRepo"], json!("IAM"));
    }

    #[test]
    fn no_overlay_returns_base_unchanged() {
        let base = json!({ "storyPoints": 5 });
        let result = merge(Some(base.clone()), None);
        assert_eq!(result, base);
    }

    // --- HTTP handler integration tests ---

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::http_method::HttpMethod;
    use crate::spec::RouteConfig;

    use super::build;

    fn route(method: HttpMethod, path: &str, status: u16, body: Option<Value>) -> RouteConfig {
        RouteConfig {
            axum_path: path.to_string(),
            method,
            status_code: status,
            body,
            discriminator_field: None,
            variants: None,
        }
    }

    async fn send(app: axum::Router, req: Request<Body>) -> axum::response::Response {
        ServiceExt::<Request<Body>>::oneshot(app, req)
            .await
            .unwrap()
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn get_returns_precomputed_body_with_200() {
        let app = build(vec![route(
            HttpMethod::Get,
            "/items",
            200,
            Some(json!({"id": "abc"})),
        )]);
        let response = send(
            app,
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["id"], json!("abc"));
    }

    #[tokio::test]
    async fn delete_returns_204_with_no_meaningful_body() {
        let app = build(vec![route(HttpMethod::Delete, "/items/{id}", 204, None)]);
        let response = send(
            app,
            Request::builder()
                .method(Method::DELETE)
                .uri("/items/123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = build(vec![route(HttpMethod::Get, "/items", 200, Some(json!({})))]);
        let response = send(
            app,
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_dispatches_to_correct_variant_by_discriminator() {
        let app = build(vec![RouteConfig {
            axum_path: "/items".to_string(),
            method: HttpMethod::Post,
            status_code: 201,
            body: None,
            discriminator_field: Some("kind".to_string()),
            variants: Some(
                [
                    ("a".to_string(), json!({"kind": "a", "value": 1})),
                    ("b".to_string(), json!({"kind": "b", "value": 2})),
                ]
                .into(),
            ),
        }]);
        let response = send(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"b"}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response_json(response).await;
        assert_eq!(body["kind"], json!("b"));
        assert_eq!(body["value"], json!(2));
    }

    #[tokio::test]
    async fn post_falls_back_to_first_variant_for_unknown_discriminator_value() {
        let app = build(vec![RouteConfig {
            axum_path: "/items".to_string(),
            method: HttpMethod::Post,
            status_code: 201,
            body: None,
            discriminator_field: Some("kind".to_string()),
            variants: Some([("a".to_string(), json!({"kind": "a", "fromVariantA": true}))].into()),
        }]);
        let response = send(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"unknown"}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response_json(response).await["fromVariantA"], json!(true));
    }

    #[tokio::test]
    async fn post_request_body_fields_override_variant() {
        let app = build(vec![RouteConfig {
            axum_path: "/items".to_string(),
            method: HttpMethod::Post,
            status_code: 201,
            body: None,
            discriminator_field: Some("kind".to_string()),
            variants: Some([("a".to_string(), json!({"kind": "a", "score": 5}))].into()),
        }]);
        let response = send(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"a","score":9}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(response_json(response).await["score"], json!(9));
    }

    #[tokio::test]
    async fn patch_request_body_fields_override_base() {
        let app = build(vec![route(
            HttpMethod::Patch,
            "/items/{id}",
            200,
            Some(json!({"id": "abc", "status": "draft"})),
        )]);
        let response = send(
            app,
            Request::builder()
                .method(Method::PATCH)
                .uri("/items/abc")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"status":"active"}"#))
                .unwrap(),
        )
        .await;
        let body = response_json(response).await;
        assert_eq!(body["id"], json!("abc"));
        assert_eq!(body["status"], json!("active"));
    }

    #[tokio::test]
    async fn options_route_returns_precomputed_body() {
        let app = build(vec![route(
            HttpMethod::Options,
            "/items",
            200,
            Some(json!({"ok": true})),
        )]);
        let response = send(
            app,
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["ok"], json!(true));
    }

    #[tokio::test]
    async fn head_route_returns_200() {
        let app = build(vec![route(
            HttpMethod::Head,
            "/items",
            200,
            Some(json!({})),
        )]);
        let response = send(
            app,
            Request::builder()
                .method(Method::HEAD)
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn trace_route_returns_200() {
        let app = build(vec![route(
            HttpMethod::Trace,
            "/items",
            200,
            Some(json!({})),
        )]);
        let response = send(
            app,
            Request::builder()
                .method(Method::TRACE)
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_and_post_on_same_path_both_work() {
        let app = build(vec![
            route(
                HttpMethod::Get,
                "/items",
                200,
                Some(json!({"method": "get"})),
            ),
            route(HttpMethod::Post, "/items", 201, None),
        ]);

        let get_response = send(
            app.clone(),
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(get_response.status(), StatusCode::OK);
        assert_eq!(response_json(get_response).await["method"], json!("get"));

        let post_response = send(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;
        assert_eq!(post_response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn readonly_handler_returns_404_when_route_not_in_state() {
        use super::{AppState, handle_readonly};
        use axum::Router;
        use std::collections::HashMap;
        use std::sync::Arc;
        let state = Arc::new(AppState {
            routes: HashMap::new(),
        });
        let app = Router::new()
            .route("/items", axum::routing::get(handle_readonly))
            .with_state(state);
        let response = send(
            app,
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn body_handler_returns_404_when_route_not_in_state() {
        use super::{AppState, handle_with_body};
        use axum::Router;
        use std::collections::HashMap;
        use std::sync::Arc;
        let state = Arc::new(AppState {
            routes: HashMap::new(),
        });
        let app = Router::new()
            .route("/items", axum::routing::post(handle_with_body))
            .with_state(state);
        let response = send(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn put_request_body_fields_override_base() {
        let app = build(vec![route(
            HttpMethod::Put,
            "/items/{id}",
            200,
            Some(json!({"id": "abc", "name": "old"})),
        )]);
        let response = send(
            app,
            Request::builder()
                .method(Method::PUT)
                .uri("/items/abc")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"new"}"#))
                .unwrap(),
        )
        .await;
        let body = response_json(response).await;
        assert_eq!(body["id"], json!("abc"));
        assert_eq!(body["name"], json!("new"));
    }
}

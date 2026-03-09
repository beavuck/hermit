use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{MatchedPath, State},
    http::{Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::MethodRouter,
};

use crate::constants::{DEFAULT_MAX_ITEMS, DEFAULT_MIN_ITEMS};
use crate::http_method::HttpMethod;
use crate::resource_store::{
    CrudStore, build_collection_response, extract_items_from_mock, fill_to_count, is_item_pattern,
    new_uuid,
};
use crate::spec_parser::RouteConfig;

#[derive(Clone)]
pub struct AppState {
    routes: HashMap<String, RouteConfig>,
    store: Arc<Mutex<CrudStore>>,
    collection_templates: HashMap<String, Vec<serde_json::Value>>,
    min_items: usize,
    max_items: usize,
}

pub fn build(configs: Vec<RouteConfig>) -> Router {
    build_with_bounds(configs, DEFAULT_MIN_ITEMS, DEFAULT_MAX_ITEMS)
}

pub fn build_with_bounds(configs: Vec<RouteConfig>, min_items: usize, max_items: usize) -> Router {
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

    let mut collection_templates: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for cfg in &configs {
        if cfg.method == HttpMethod::Get && !is_item_pattern(&cfg.axum_path) {
            collection_templates.insert(cfg.axum_path.clone(), extract_items_from_mock(&cfg.body));
        }
    }

    let state = Arc::new(AppState {
        routes: into_state_map(configs),
        store: Arc::new(Mutex::new(CrudStore::new())),
        collection_templates,
        min_items,
        max_items,
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
    uri: Uri,
    method: Method,
) -> Response {
    let key = route_key(method.as_str(), matched.as_str());
    let cfg = match state.routes.get(&key) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let concrete = uri.path();

    match method {
        Method::GET if is_item_pattern(matched.as_str()) => get_item(&state, cfg, concrete),
        Method::GET => get_collection(&state, cfg, concrete),
        Method::DELETE => delete_item(&state, cfg, concrete),
        _ => {
            if cfg.status_code == StatusCode::NO_CONTENT.as_u16() {
                StatusCode::NO_CONTENT.into_response()
            } else {
                json_response(cfg.status_code, cfg.body.clone())
            }
        }
    }
}

async fn handle_with_body(
    State(state): State<Arc<AppState>>,
    matched: MatchedPath,
    uri: Uri,
    method: Method,
    body: Option<Json<serde_json::Value>>,
) -> Response {
    let key = route_key(method.as_str(), matched.as_str());
    let cfg = match state.routes.get(&key) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let request_fields = body.map(|Json(b)| b);
    let concrete = uri.path();

    match method {
        Method::POST => post_item(&state, cfg, concrete, request_fields),
        _ => put_or_patch(&state, cfg, concrete, request_fields, &method),
    }
}

fn get_item(state: &AppState, cfg: &RouteConfig, concrete: &str) -> Response {
    let id = concrete.rsplit('/').next().unwrap_or("").to_string();
    let mut fallback = cfg.body.clone().unwrap_or(serde_json::Value::Null);
    if let Some(obj) = fallback.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id));
    }

    let mut store = state.store.lock().unwrap();
    store.seed_item(concrete, fallback);
    json_response(cfg.status_code, store.get_item(concrete).cloned())
}

fn get_collection(state: &AppState, cfg: &RouteConfig, concrete: &str) -> Response {
    let mut store = state.store.lock().unwrap();
    if !store.collection_initialized(concrete) {
        let templates = state
            .collection_templates
            .get(&cfg.axum_path)
            .cloned()
            .unwrap_or_default();
        let filled = fill_to_count(templates, state.min_items, state.max_items);
        for item in filled {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(new_uuid);
            store.seed_item(&format!("{}/{}", concrete, id), item);
        }
        store.init_collection(concrete);
    }
    let items = store.collection_items(concrete).unwrap_or_default();
    let body = build_collection_response(&cfg.body, items);
    json_response(cfg.status_code, Some(body))
}

fn delete_item(state: &AppState, cfg: &RouteConfig, concrete: &str) -> Response {
    state.store.lock().unwrap().delete_item(concrete);
    if cfg.status_code == StatusCode::NO_CONTENT.as_u16() {
        StatusCode::NO_CONTENT.into_response()
    } else {
        json_response(cfg.status_code, cfg.body.clone())
    }
}

fn post_item(
    state: &AppState,
    cfg: &RouteConfig,
    collection: &str,
    request_fields: Option<serde_json::Value>,
) -> Response {
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

    let new_item = merge(base, request_fields);
    let id = new_item
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(new_uuid);
    let item_path = format!("{}/{}", collection, id);

    let mut store = state.store.lock().unwrap();
    store.put_item(&item_path, new_item.clone());
    json_response(cfg.status_code, Some(new_item))
}

fn put_or_patch(
    state: &AppState,
    cfg: &RouteConfig,
    concrete: &str,
    request_fields: Option<serde_json::Value>,
    method: &Method,
) -> Response {
    let base = if *method == Method::PATCH {
        let store = state.store.lock().unwrap();
        store
            .get_item(concrete)
            .cloned()
            .or_else(|| cfg.body.clone())
    } else {
        cfg.body.clone()
    };

    let updated = merge(base, request_fields);
    state
        .store
        .lock()
        .unwrap()
        .put_item(concrete, updated.clone());
    json_response(cfg.status_code, Some(updated))
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
    use crate::spec_parser::RouteConfig;

    use super::{build, build_with_bounds};

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
        use crate::constants::{DEFAULT_MAX_ITEMS, DEFAULT_MIN_ITEMS};
        use crate::resource_store::CrudStore;
        use axum::Router;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        let state = Arc::new(AppState {
            routes: HashMap::new(),
            store: Arc::new(Mutex::new(CrudStore::new())),
            collection_templates: HashMap::new(),
            min_items: DEFAULT_MIN_ITEMS,
            max_items: DEFAULT_MAX_ITEMS,
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
        use crate::constants::{DEFAULT_MAX_ITEMS, DEFAULT_MIN_ITEMS};
        use crate::resource_store::CrudStore;
        use axum::Router;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        let state = Arc::new(AppState {
            routes: HashMap::new(),
            store: Arc::new(Mutex::new(CrudStore::new())),
            collection_templates: HashMap::new(),
            min_items: DEFAULT_MIN_ITEMS,
            max_items: DEFAULT_MAX_ITEMS,
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

    // --- CRUD state tests ---

    #[tokio::test]
    async fn get_item_reflects_stored_value_from_prior_put() {
        let mock = Some(json!({"id": "m", "name": "mock"}));
        let app = build(vec![
            route(HttpMethod::Get, "/items/{id}", 200, mock.clone()),
            route(HttpMethod::Put, "/items/{id}", 200, mock),
        ]);
        send(
            app.clone(),
            Request::builder()
                .method(Method::PUT)
                .uri("/items/abc")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"updated"}"#))
                .unwrap(),
        )
        .await;
        let response = send(
            app,
            Request::builder()
                .uri("/items/abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response_json(response).await["name"], json!("updated"));
    }

    #[tokio::test]
    async fn get_item_id_matches_path_segment_on_first_access() {
        let app = build(vec![route(
            HttpMethod::Get,
            "/items/{id}",
            200,
            Some(json!({"id": "mock-uuid", "name": "x"})),
        )]);
        let response = send(
            app,
            Request::builder()
                .uri("/items/my-specific-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response_json(response).await["id"], json!("my-specific-id"));
    }

    #[tokio::test]
    async fn post_item_is_retrievable_via_get_item() {
        let app = build(vec![
            route(
                HttpMethod::Post,
                "/items",
                201,
                Some(json!({"id": "mock-id", "name": ""})),
            ),
            route(
                HttpMethod::Get,
                "/items/{id}",
                200,
                Some(json!({"id": "mock-id", "name": "mock"})),
            ),
        ]);
        let post_resp = send(
            app.clone(),
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"created"}"#))
                .unwrap(),
        )
        .await;
        let post_body = response_json(post_resp).await;
        let new_id = post_body["id"].as_str().unwrap().to_string();
        let get_resp = send(
            app,
            Request::builder()
                .uri(format!("/items/{}", new_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response_json(get_resp).await["name"], json!("created"));
    }

    #[tokio::test]
    async fn post_item_appears_in_get_collection() {
        let app = build(vec![
            route(
                HttpMethod::Post,
                "/items",
                201,
                Some(json!({"id": "new-id", "name": ""})),
            ),
            route(
                HttpMethod::Get,
                "/items",
                200,
                Some(json!({"total": 0, "items": []})),
            ),
        ]);
        send(
            app.clone(),
            Request::builder()
                .method(Method::POST)
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"new"}"#))
                .unwrap(),
        )
        .await;
        let response = send(
            app,
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let body = response_json(response).await;
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], json!("new"));
    }

    #[tokio::test]
    async fn patch_preserves_fields_not_in_request_body_from_prior_put() {
        let mock = Some(json!({"id": "m", "name": "mock", "color": "mock"}));
        let app = build(vec![
            route(HttpMethod::Get, "/items/{id}", 200, mock.clone()),
            route(HttpMethod::Put, "/items/{id}", 200, mock.clone()),
            route(HttpMethod::Patch, "/items/{id}", 200, mock),
        ]);
        send(
            app.clone(),
            Request::builder()
                .method(Method::PUT)
                .uri("/items/abc")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"from-put","color":"red"}"#))
                .unwrap(),
        )
        .await;
        send(
            app.clone(),
            Request::builder()
                .method(Method::PATCH)
                .uri("/items/abc")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"color":"blue"}"#))
                .unwrap(),
        )
        .await;
        let response = send(
            app,
            Request::builder()
                .uri("/items/abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let body = response_json(response).await;
        assert_eq!(body["name"], json!("from-put"));
        assert_eq!(body["color"], json!("blue"));
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

    // --- multi-item collection tests ---

    #[tokio::test]
    async fn get_collection_default_item_count_is_between_one_and_twenty_four() {
        let app = build(vec![route(
            HttpMethod::Get,
            "/items",
            200,
            Some(json!([{"id": "seed", "name": "x"}])),
        )]);
        let response = send(
            app,
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let count = response_json(response).await.as_array().unwrap().len();
        assert!(
            (1..=24).contains(&count),
            "expected 1..=24 items, got {count}"
        );
    }

    #[tokio::test]
    async fn get_collection_item_count_is_within_explicitly_configured_bounds() {
        let app = build_with_bounds(
            vec![route(
                HttpMethod::Get,
                "/items",
                200,
                Some(json!([{"id": "seed", "name": "x"}])),
            )],
            5,
            5,
        );
        let response = send(
            app,
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response_json(response).await.as_array().unwrap().len(), 5);
    }
}

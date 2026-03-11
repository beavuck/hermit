use axum::body::Body;
use axum::http::Request;
use criterion::{Criterion, criterion_group, criterion_main};
use hermit::{router, spec_parser};
use http_body_util::BodyExt;
use tower::ServiceExt;

const SPEC_PATH: &str = "specs_assets/taskflow.openapi.yml";

fn build_router() -> axum::Router {
    let spec = spec_parser::load(std::path::Path::new(SPEC_PATH));
    let routes = spec_parser::extract_routes(&spec);
    router::build(routes)
}

fn bench_request_get_collection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = build_router();
    c.bench_function("request_get_collection", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Request::builder()
                    .uri("/projects")
                    .body(Body::empty())
                    .unwrap();
                let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), req)
                    .await
                    .unwrap();
                response.into_body().collect().await.unwrap()
            })
        })
    });
}

fn bench_request_get_item(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = build_router();
    c.bench_function("request_get_item", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Request::builder()
                    .uri("/projects/c9d8e7f6-a5b4-3210-fedc-ba9876543210")
                    .body(Body::empty())
                    .unwrap();
                let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), req)
                    .await
                    .unwrap();
                response.into_body().collect().await.unwrap()
            })
        })
    });
}

fn bench_request_post_with_echo(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = build_router();
    c.bench_function("request_post_with_echo", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Request::builder()
                    .method("POST")
                    .uri("/projects")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"bench"}"#))
                    .unwrap();
                let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), req)
                    .await
                    .unwrap();
                response.into_body().collect().await.unwrap()
            })
        })
    });
}

criterion_group!(
    benches,
    bench_request_get_collection,
    bench_request_get_item,
    bench_request_post_with_echo,
);
criterion_main!(benches);

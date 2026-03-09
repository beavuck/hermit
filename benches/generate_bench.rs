use criterion::{Criterion, criterion_group, criterion_main};
use hermit::{generator, spec};

const SPEC_PATH: &str = "specs_assets/taskflow.openapi.yml";

fn bench_generate(c: &mut Criterion) {
    let root = spec::load(std::path::Path::new(SPEC_PATH));
    let routes = spec::extract_routes(&root);

    let route_with_body = routes
        .iter()
        .find(|r| r.body.is_some())
        .expect("expected at least one route with a generated body");

    let schema = root["paths"][&route_with_body.axum_path][route_with_body.method.as_str()]["responses"]
        ["200"]["content"]["application/json"]["schema"]
        .clone();

    c.bench_function("generator_generate", |b| {
        b.iter(|| {
            generator::generate(
                std::hint::black_box(&schema),
                std::hint::black_box(&root),
                None,
            )
        })
    });
}

criterion_group!(benches, bench_generate);
criterion_main!(benches);

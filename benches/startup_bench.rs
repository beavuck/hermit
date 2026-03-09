use criterion::{Criterion, criterion_group, criterion_main};
use hermit::spec_parser;

const SPEC_PATH: &str = "specs_assets/taskflow.openapi.yml";

fn bench_load(c: &mut Criterion) {
    let path = std::path::Path::new(SPEC_PATH);
    c.bench_function("spec_load", |b| {
        b.iter(|| spec_parser::load(std::hint::black_box(path)))
    });
}

fn bench_extract_routes(c: &mut Criterion) {
    let spec = spec_parser::load(std::path::Path::new(SPEC_PATH));
    c.bench_function("spec_extract_routes", |b| {
        b.iter(|| spec_parser::extract_routes(std::hint::black_box(&spec)))
    });
}

criterion_group!(benches, bench_load, bench_extract_routes);
criterion_main!(benches);

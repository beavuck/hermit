use criterion::{Criterion, criterion_group, criterion_main};
use hermit::spec_parser;

const TASKFLOW_PATH: &str = "specs_assets/taskflow.openapi.yml";
const DOG_CAFE_PATH: &str = "specs_assets/dog_cafe.openapi.yml";

fn bench_load_taskflow(c: &mut Criterion) {
    let path = std::path::Path::new(TASKFLOW_PATH);
    c.bench_function("spec_load_taskflow", |b| {
        b.iter(|| spec_parser::load(std::hint::black_box(path)))
    });
}

fn bench_extract_routes(c: &mut Criterion) {
    let spec = spec_parser::load(std::path::Path::new(TASKFLOW_PATH));
    c.bench_function("spec_extract_routes", |b| {
        b.iter(|| spec_parser::extract_routes(std::hint::black_box(&spec)))
    });
}

fn bench_load_dog_cafe(c: &mut Criterion) {
    let path = std::path::Path::new(DOG_CAFE_PATH);
    c.bench_function("spec_load_dog_cafe", |b| {
        b.iter(|| spec_parser::load(std::hint::black_box(path)))
    });
}

fn bench_extract_routes_dog_cafe(c: &mut Criterion) {
    let spec = spec_parser::load(std::path::Path::new(DOG_CAFE_PATH));
    c.bench_function("spec_extract_routes_dog_cafe", |b| {
        b.iter(|| spec_parser::extract_routes(std::hint::black_box(&spec)))
    });
}

fn bench_load_all_parallel(c: &mut Criterion) {
    let paths = vec![
        std::path::PathBuf::from(TASKFLOW_PATH),
        std::path::PathBuf::from(DOG_CAFE_PATH),
    ];
    c.bench_function("spec_load_all_parallel", |b| {
        b.iter(|| spec_parser::load_all(std::hint::black_box(&paths)))
    });
}

fn bench_load_all_sequential(c: &mut Criterion) {
    c.bench_function("spec_load_all_sequential", |b| {
        b.iter(|| {
            let taskflow = spec_parser::load(std::path::Path::new(TASKFLOW_PATH));
            let _routes_tf = spec_parser::extract_routes(&taskflow);
            let dog_cafe = spec_parser::load(std::path::Path::new(DOG_CAFE_PATH));
            let _routes_dc = spec_parser::extract_routes(&dog_cafe);
        })
    });
}

criterion_group!(
    benches,
    bench_load_taskflow,
    bench_extract_routes,
    bench_load_dog_cafe,
    bench_extract_routes_dog_cafe,
    bench_load_all_parallel,
    bench_load_all_sequential,
);
criterion_main!(benches);

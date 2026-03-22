use beavuck_hermit::{resource_generator, spec_loader};
use criterion::{Criterion, criterion_group, criterion_main};

const SPEC_PATH: &str = "specs_assets/taskflow.openapi.yml";

fn bench_generate_plain_object(c: &mut Criterion) {
    let root = spec_loader::load(std::path::Path::new(SPEC_PATH));
    let schema = root["components"]["schemas"]["ProjectBase"].clone();
    c.bench_function("generate_plain_object", |b| {
        b.iter(|| {
            resource_generator::generate(
                std::hint::black_box(&schema),
                std::hint::black_box(&root),
                None,
            )
        })
    });
}

fn bench_generate_deep_all_of(c: &mut Criterion) {
    let root = spec_loader::load(std::path::Path::new(SPEC_PATH));
    let schema = root["components"]["schemas"]["Project"].clone();
    c.bench_function("generate_deep_all_of", |b| {
        b.iter(|| {
            resource_generator::generate(
                std::hint::black_box(&schema),
                std::hint::black_box(&root),
                None,
            )
        })
    });
}

fn bench_generate_discriminator_forced(c: &mut Criterion) {
    let root = spec_loader::load(std::path::Path::new(SPEC_PATH));
    let schema = root["components"]["schemas"]["TaskCreate"].clone();
    c.bench_function("generate_discriminator_forced", |b| {
        b.iter(|| {
            resource_generator::generate(
                std::hint::black_box(&schema),
                std::hint::black_box(&root),
                Some("bug"),
            )
        })
    });
}

fn bench_generate_paginated_array(c: &mut Criterion) {
    let root = spec_loader::load(std::path::Path::new(SPEC_PATH));
    let schema = root["components"]["schemas"]["ProjectPage"].clone();
    c.bench_function("generate_paginated_array", |b| {
        b.iter(|| {
            resource_generator::generate(
                std::hint::black_box(&schema),
                std::hint::black_box(&root),
                None,
            )
        })
    });
}

criterion_group!(
    benches,
    bench_generate_plain_object,
    bench_generate_deep_all_of,
    bench_generate_discriminator_forced,
    bench_generate_paginated_array,
);
criterion_main!(benches);

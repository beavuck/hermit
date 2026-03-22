setup:
    cargo install cargo-llvm-cov critcmp cargo-edit cargo-zigbuild
    cargo install --locked cargo-nextest

run:
    cargo run -- --ignore-examples --specs-dir specs_assets

run-huge:
    cargo run -- --ignore-examples --specs-dir huge_specs

bench:
    cargo nextest bench --bench startup_bench --bench generate_bench --bench request_bench

bench-ci:
    cargo nextest bench --bench startup_bench --bench generate_bench --bench request_bench -- --save-baseline change

test:
    cargo nextest run --no-fail-fast

lint:
    cargo clippy --fix --all-targets --allow-dirty --allow-staged
    cargo fmt --all

mutants:
    cargo mutants --test-tool=nextest

HERMIT_IGNORE_COVERAGE := env_var_or_default('HERMIT_IGNORE_COVERAGE', 'main\.rs|http_method\.rs')

coverage:
    cargo llvm-cov nextest --html --ignore-filename-regex "{{ HERMIT_IGNORE_COVERAGE }}"

coverage-lcov:
    cargo llvm-cov nextest --lcov --output-path coverage/lcov.info --ignore-filename-regex "{{ HERMIT_IGNORE_COVERAGE }}"

coverage-ci:
    cargo llvm-cov nextest --no-report
    cargo llvm-cov report --lcov --output-path lcov.info --ignore-filename-regex "{{ HERMIT_IGNORE_COVERAGE }}"

upgrade-ci:
    cargo install cargo-edit
    cargo upgrade --verbose --incompatible
    cargo update
    cargo set-version --bump patch

pre-commit:
    just lint
    just test

up-major:
    cargo set-version --bump major

up-minor:
    cargo set-version --bump minor

up-patch:
    cargo set-version --bump patch

RUST_TARGET := env_var_or_default('RUST_TARGET', 'x86_64-unknown-linux-musl')

build-release:
    rustup target add "{{ RUST_TARGET }}" && cargo zigbuild --release --target "{{ RUST_TARGET }}"

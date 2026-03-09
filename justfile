setup:
    cargo install cargo-llvm-cov critcmp cargo-edit

bench:
    cargo bench --bench startup_bench --bench generate_bench

bench-ci:
    cargo bench --bench startup_bench --bench generate_bench -- --save-baseline change

test:
    cargo test

lint:
    cargo fmt --all
    cargo clippy --fix --all-targets

coverage:
    cargo llvm-cov --html --ignore-filename-regex "main\.rs|http_method\.rs"

coverage-lcov:
    cargo llvm-cov --lcov --output-path coverage/lcov.info --ignore-filename-regex "main\.rs|http_method\.rs"

coverage-ci:
    cargo llvm-cov --no-report
    cargo llvm-cov report --lcov --output-path lcov.info --ignore-filename-regex "main\.rs|http_method\.rs"

upgrade-ci:
    cargo upgrade --verbose --incompatible
    cargo update
    cargo set-version --bump patch

pre-commit:
    just lint
    just test

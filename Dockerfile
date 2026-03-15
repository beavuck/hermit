FROM dhi.io/rust:1-alpine3.23-dev AS builder
RUN apk add --no-cache rustup && rustup-init -y
ENV PATH="/root/.cargo/bin:$PATH"
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
# Needed by Cargo.toml
COPY benches/ benches/

ENV RUSTFLAGS="-C target-feature=+crt-static"
# Produces a statically-linked x86_64 (linux/amd64) binary. Not portable to other architectures.
RUN cargo build --release --target x86_64-unknown-linux-musl


FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/hermit /hermit

USER 1000:1000
EXPOSE 8532
VOLUME ["/specs"]
ENTRYPOINT ["/hermit"]

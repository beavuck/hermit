FROM --platform=$BUILDPLATFORM dhi.io/rust:1-alpine3.23-dev AS builder
ARG TARGETARCH

ENV PATH="/root/.cargo/bin:$PATH"
RUN apk add --no-cache rustup zig && rustup-init -y \
    && cargo install cargo-zigbuild \
    && case "$TARGETARCH" in \
      amd64) echo "x86_64-unknown-linux-musl" ;; \
      arm64) echo "aarch64-unknown-linux-musl" ;; \
      *) echo "Unsupported architecture: $TARGETARCH" >&2 && exit 1 ;; \
    esac > /rust_target \
    && rustup target add "$(cat /rust_target)"

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
# Needed by Cargo.toml
COPY benches/ benches/

ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo zigbuild --release --target "$(cat /rust_target)" \
    && cp "target/$(cat /rust_target)/release/hermit" /hermit-bin


FROM scratch

COPY --from=builder /hermit-bin /hermit

USER 1000:1000
EXPOSE 8532
VOLUME ["/specs"]
ENTRYPOINT ["/hermit"]

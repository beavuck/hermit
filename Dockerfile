FROM --platform=$BUILDPLATFORM dhi.io/rust:1-alpine3.23-dev AS builder
ARG TARGETARCH

RUN apk add --no-cache rustup zig && rustup-init -y
ENV PATH="/root/.cargo/bin:$PATH"
RUN cargo install cargo-zigbuild

RUN case "$TARGETARCH" in \
      amd64) echo "x86_64-unknown-linux-musl" ;; \
      arm64) echo "aarch64-unknown-linux-musl" ;; \
      *) echo "Unsupported architecture: $TARGETARCH" >&2 && exit 1 ;; \
    esac > /rust_target

RUN rustup target add $(cat /rust_target)

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
# Needed by Cargo.toml
COPY benches/ benches/

ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo zigbuild --release --target $(cat /rust_target)
RUN cp target/$(cat /rust_target)/release/hermit /hermit-bin


FROM scratch

COPY --from=builder /hermit-bin /hermit

USER 1000:1000
EXPOSE 8532
VOLUME ["/specs"]
ENTRYPOINT ["/hermit"]

# Docker

Hermit is published to DockerHub as `beavuck/hermit`. The image is a statically-linked binary running on a `scratch`
base -- no OS, no shell, minimal attack surface. It supports `linux/amd64` and `linux/arm64`.

## Building locally

To build the docker image locally, do make sure to auth first, as we're using a Docker Hardened Image:

```bash
echo "$DOCKERHUB_WRITE_TOKEN" | docker login --username "$DOCKERHUB_USERNAME" --password-stdin
```

To build and load a single-platform image for local testing:

```bash
docker build --load --tag "beavuck/hermit" .
```

To verify the multi-platform build locally, you need a buildx builder that uses the `docker-container` driver (the
default `docker` driver does not support multi-platform). Create one once:

```bash
docker buildx create --name multiarch --use
```

Multi-platform images cannot be loaded into the local Docker daemon -- they must be pushed directly to a registry or
stay
in the build cache:

```bash
docker buildx build --platform linux/amd64,linux/arm64 --tag "beavuck/hermit" .
```

To also push, you may add `--push`. But the CI pipeline should take care of that, typically.

## Running

Mount your OpenAPI spec files into `/specs`. Use `HERMIT_SPECS_DIR` to load all files in a directory:

```sh
docker run --rm --pull=always \
  -p 8532:8532 \
  -v ./specs_assets:/specs:ro \
  -e HERMIT_SPECS_DIR=specs \
  beavuck/hermit
```

Or use `HERMIT_SPECS` to load specific files (comma-separated):

```sh
docker run --rm --pull=always \
  -p 8532:8532 \
  -v ./specs_assets:/specs:ro \
  -e HERMIT_SPECS=specs/taskflow.openapi.yml,specs/dog_cafe.openapi.json \
  beavuck/hermit
```

`HERMIT_SPECS_DIR` and `HERMIT_SPECS` are mutually exclusive -- exactly one must be provided.

## Environment variables

All CLI flags are also available as environment variables. CLI args take precedence over env vars.

| Variable                 | CLI equivalent      | Default      |
|--------------------------|---------------------|--------------|
| `HERMIT_SPECS_DIR`       | `--specs-dir`       | *(required)* |
| `HERMIT_SPECS`           | `--specs`           | *(required)* |
| `HERMIT_PORT`            | `--port`            | `8532`       |
| `HERMIT_MIN_ITEMS`       | `--min-items`       | `1`          |
| `HERMIT_MAX_ITEMS`       | `--max-items`       | `20`         |
| `HERMIT_IGNORE_EXAMPLES` | `--ignore-examples` | `false`      |

`HERMIT_SPECS_DIR` and `HERMIT_SPECS` are mutually exclusive -- exactly one must be provided.
`HERMIT_SPECS` accepts a comma-separated list of paths.

## docker-compose

Here is a docker-compose example of a typical Hermit setup:

```yaml
  hermit:
    image: beavuck/hermit:latest
    environment:
      HERMIT_SPECS_DIR: specs
      HERMIT_IGNORE_EXAMPLES: true
    ports:
      - "8532:8532"
    volumes:
      - ./specs_assets:/specs:ro
```

## Design notes

**`scratch` base** -- the binary is compiled with `RUSTFLAGS="-C target-feature=+crt-static"` targeting a musl Linux
triple, producing a fully static binary with no libc dependency.

**Multi-platform build** -- the Dockerfile uses `--platform=$BUILDPLATFORM` on the builder stage so it always runs
natively on the CI host, regardless of the target architecture. `ARG TARGETARCH` receives the target from
`docker buildx` (`amd64` or `arm64`) and is mapped to the corresponding Rust target triple. `cargo-zigbuild` uses zig
as a cross-compiler, eliminating the need for platform-specific C toolchains.

**Fixed binary path before `scratch`** -- the builder copies the binary to `/hermit-bin` before the final stage.
`scratch` has no shell, so `COPY --from=builder` cannot evaluate `$(cat /rust_target)`. A fixed path sidesteps this.

**PID 1 signal handling** -- Linux gives PID 1 special treatment: it ignores signals unless it explicitly handles them.
Without a signal handler, `docker stop` would wait 10 seconds then force-kill the container. Hermit registers handlers
for both SIGTERM and SIGINT so it shuts down immediately.

**`--target` flag and proc-macros** -- building without an explicit target on the Alpine builder uses
`x86_64-alpine-linux-musl` as the host target, which applies `RUSTFLAGS` to all crates including proc-macros.
Proc-macro crates must be compiled as dylibs, which is incompatible with `crt-static` on musl. Specifying `--target`
separates host builds (proc-macros, no RUSTFLAGS) from target builds (the binary, with RUSTFLAGS).

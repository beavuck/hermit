# Docker

Hermit is published to DockerHub as `beavuck/hermit`. The image is a statically-linked binary running on a `scratch`
base — no OS, no shell, minimal attack surface.

## Building locally

> **Architecture:** the image targets `linux/amd64` (x86_64) only. Building or running on ARM is not yet supported.

To build the docker image locally, do make sure to auth first, as we're using a Docker Hardened Image:

```bash
echo "$DOCKERHUB_WRITE_TOKEN" | docker login --username "$DOCKERHUB_USERNAME" --password-stdin
docker build --tag "beavuck/hermit" .
```

## Running

Mount your OpenAPI spec files into `/specs` and point `HERMIT_SPECS` at them:

```sh
docker run --rm \
  -p 8532:8532 \
  -v ./specs_assets:/specs:ro \
  -e HERMIT_SPECS=specs/taskflow.openapi.yml,specs/dog_cafe.openapi.yml \
  beavuck/hermit
```

Replace `specs_assets` with the path to your spec files.

Replace the `*.openapi.yml` files with your actual files.

## Environment variables

All CLI flags are also available as environment variables. CLI args take precedence over env vars.

| Variable                 | CLI equivalent      | Default      |
|--------------------------|---------------------|--------------|
| `HERMIT_SPECS`           | `--specs`           | *(required)* |
| `HERMIT_PORT`            | `--port`            | `8532`       |
| `HERMIT_MIN_ITEMS`       | `--min-items`       | `1`          |
| `HERMIT_MAX_ITEMS`       | `--max-items`       | `20`         |
| `HERMIT_IGNORE_EXAMPLES` | `--ignore-examples` | `false`      |

`HERMIT_SPECS` accepts a comma-separated list of paths.

## docker-compose

Here is a docker-compose example of a typical Hermit setup:

```yaml
  hermit:
    image: beavuck/hermit:latest
    environment:
      HERMIT_SPECS: specs/taskflow.openapi.yml,specs/dog_cafe.openapi.yml
      HERMIT_IGNORE_EXAMPLES: true
    ports:
      - "8532:8532"
    volumes:
      - ./specs_assets:/specs:ro
```

## Design notes

**`scratch` base** — the binary is compiled with `RUSTFLAGS="-C target-feature=+crt-static"` targeting
`x86_64-unknown-linux-musl`, producing a fully static binary with no libc dependency. This is why `scratch` works and is
preferred over Alpine.

**PID 1 signal handling** — Linux gives PID 1 special treatment: it ignores signals unless it explicitly handles them.
Without a signal handler, `docker stop` would wait 10 seconds then force-kill the container. Hermit registers handlers
for both SIGTERM and SIGINT so it shuts down immediately.

**`--target x86_64-unknown-linux-musl` in the Dockerfile** — building without an explicit target on the Alpine builder
uses `x86_64-alpine-linux-musl` as the host target, which applies `RUSTFLAGS` to all crates including proc-macros.
Proc-macro crates must be compiled as dylibs, which is incompatible with `crt-static` on musl. Specifying `--target`
separates host builds (proc-macros, no RUSTFLAGS) from target builds (the binary, with RUSTFLAGS).

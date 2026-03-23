# 🦀🐚 Beavuck Hermit

Zero-config OpenAPI mock server. Point it at a spec file, and it instantly serves schema-accurate responses -- no stubs
to write, no configuration.

Supports `linux/amd64` and `linux/arm64`.

## Quick start

Mount your specs directory and point `HERMIT_SPECS_DIR` at it:

```sh
docker run --rm --pull=always \
  -p 8532:8532 \
  -v ./your-specs:/specs:ro \
  -e HERMIT_SPECS_DIR=specs \
  beavuck/hermit
```

Your mock server is now running on port `8532`.

## docker-compose

```yaml
hermit:
  image: beavuck/hermit:latest
  pull_policy: always
  environment:
    HERMIT_SPECS_DIR: specs
  ports:
    - "8532:8532"
  volumes:
    - ./your-specs:/specs:ro
```

## Loading specific files

To load individual files rather than a whole directory, use `HERMIT_SPECS` with a comma-separated list:

```sh
docker run --rm --pull=always \
-p 8532:8532 \
-v ./your-specs:/specs:ro \
-e HERMIT_SPECS=specs/users-api.openapi.yml \
beavuck/hermit
```

## Environment variables

| Variable                      | Default      | Description                                                                         |
|-------------------------------|--------------|-------------------------------------------------------------------------------------|
| `HERMIT_SPECS_DIR`            | *(required)* | Path to a directory of OpenAPI spec files (conflicts with `HERMIT_SPECS`)           |
| `HERMIT_SPECS`                | *(required)* | Comma-separated paths to individual spec files  (conflicts with `HERMIT_SPECS_DIR`) |
| `HERMIT_PORT`                 | `8532`       | Internal port to listen on                                                          |
| `HERMIT_MIN_ITEMS`            | `1`          | Minimum items in generated arrays                                                   |
| `HERMIT_MAX_ITEMS`            | `20`         | Maximum items in generated arrays                                                   |
| `HERMIT_USE_EXAMPLES`         | `false`      | Use `example` fields from schemas instead of generating random data                 |
| `HERMIT_CORS_ALLOWED_ORIGINS` | `*`          | Allowed CORS origins; `*` for all, or comma-separated list                          |

## How responses are generated

| Priority | Source              | When                                                                      |
|----------|---------------------|---------------------------------------------------------------------------|
| 1        | Request body fields | `POST`, `PUT`, `PATCH` -- your values are reflected back in the response  |
| 2        | `example` in schema | Field has an explicit example value (and `HERMIT_USE_EXAMPLES` is `true`) |
| 3        | `default` in schema | Field has a default value (and `HERMIT_USE_EXAMPLES` is `true`)           |
| 4        | Random value        | Fallback -- format-aware (UUID, date-time, …)                             |

Fields marked `readOnly` are never overridden by request body values. Fields marked `writeOnly` are excluded from
responses.

## Source

[gitlab.com/beavuck-services/hermit](https://gitlab.com/beavuck-services/hermit)

# Hermit

Zero-config OpenAPI mock server. Point it at a spec file, and it instantly serves schema-accurate responses — no stubs
to write, no configuration.

Supports `linux/amd64` and `linux/arm64`.

## Quick start

```sh
docker run --rm \
  -p 8532:8532 \
  -v ./your-specs:/specs:ro \
  -e HERMIT_SPECS=specs/your-api.openapi.yml \
  beavuck/hermit
```

Your mock server is now running on port `8532`.

## Loading multiple specs

Hermit can serve multiple APIs at once — just pass a comma-separated list:

```sh
docker run --rm \
  -p 8532:8532 \
  -v ./your-specs:/specs:ro \
  -e HERMIT_SPECS=specs/users-api.openapi.yml,specs/orders-api.openapi.yml \
  beavuck/hermit
```

## docker-compose

```yaml
hermit:
  image: beavuck/hermit:latest
  environment:
    HERMIT_SPECS: specs/your-api.openapi.yml
  ports:
    - "8532:8532"
  volumes:
    - ./your-specs:/specs:ro
```

## Environment variables

| Variable                 | Default      | Description                         |
|--------------------------|--------------|-------------------------------------|
| `HERMIT_SPECS`           | *(required)* | Comma-separated paths to spec files |
| `HERMIT_PORT`            | `8532`       | Port to listen on                   |
| `HERMIT_MIN_ITEMS`       | `1`          | Minimum items in generated arrays   |
| `HERMIT_MAX_ITEMS`       | `20`         | Maximum items in generated arrays   |
| `HERMIT_IGNORE_EXAMPLES` | `false`      | Ignore `example` fields in schemas  |

## How responses are generated

| Priority | Source              | When                                                                    |
|----------|---------------------|-------------------------------------------------------------------------|
| 1        | Request body fields | `POST`, `PUT`, `PATCH` — your values are reflected back in the response |
| 2        | `example` in schema | Field has an explicit example value                                     |
| 3        | `default` in schema | Field has a default value                                               |
| 4        | Random value        | Fallback — format-aware (UUID, date-time, …)                            |

Fields marked `readOnly` are never overridden by request body values. Fields marked `writeOnly` are excluded from
responses.

## Source

[gitlab.com/beavuck-services/hermit](https://gitlab.com/beavuck-services/hermit)

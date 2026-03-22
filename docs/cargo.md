# Installing and using Hermit via Cargo

## Install

```sh
cargo install hermit
```

This builds and installs the `hermit` binary to `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

To update to a newer version, run the same command again.

## Quick start

```sh
hermit --specs path/to/your-api.openapi.yml
```

Your mock server is now running on port `8532`.

## Loading specs

### A single file

```sh
hermit --specs api.openapi.yml
```

### Multiple files

```sh
hermit --specs users.openapi.yml orders.openapi.yml
```

### A whole directory

```sh
hermit --specs-dir ./specs
```

`--specs` and `--specs-dir` are mutually exclusive.

## Options

| Flag                      | Default | Description                                                       |
|---------------------------|---------|-------------------------------------------------------------------|
| `--specs <files...>`      | —       | One or more spec files to load (conflicts with `--specs-dir`)     |
| `--specs-dir <dir>`       | —       | Directory of spec files to load (conflicts with `--specs`)        |
| `--port <port>`           | `8532`  | Port to listen on                                                 |
| `--min-items <n>`         | `1`     | Minimum items in generated arrays                                 |
| `--max-items <n>`         | `20`    | Maximum items in generated arrays                                 |
| `--ignore-examples`       | `false` | Ignore `example` fields in schemas and generate random data       |
| `--cors-allowed-origins`  | `*`     | Allowed CORS origins; `*` for all, or a comma-separated list      |

Every flag can also be set via the corresponding environment variable — see below.

## Environment variables

| Variable                      | Equivalent flag              |
|-------------------------------|------------------------------|
| `HERMIT_SPECS`                | `--specs`                    |
| `HERMIT_SPECS_DIR`            | `--specs-dir`                |
| `HERMIT_PORT`                 | `--port`                     |
| `HERMIT_MIN_ITEMS`            | `--min-items`                |
| `HERMIT_MAX_ITEMS`            | `--max-items`                |
| `HERMIT_IGNORE_EXAMPLES`      | `--ignore-examples`          |
| `HERMIT_CORS_ALLOWED_ORIGINS` | `--cors-allowed-origins`     |

`HERMIT_SPECS` accepts a comma-separated list of paths.

## How responses are generated

| Priority | Source              | When                                                                     |
|----------|---------------------|--------------------------------------------------------------------------|
| 1        | Request body fields | `POST`, `PUT`, `PATCH` — your values are reflected back in the response  |
| 2        | `example` in schema | Field has an explicit example value                                      |
| 3        | `default` in schema | Field has a default value                                                |
| 4        | Random value        | Fallback — format-aware (UUID, date-time, …)                             |

Fields marked `readOnly` are never overridden by request body values. Fields marked `writeOnly` are excluded from responses.

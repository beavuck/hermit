## Options

You can always run `hermit --help` for all command-line options

| Flag                                  | Default | Description                                                                     |
|---------------------------------------|---------|---------------------------------------------------------------------------------|
| `--specs <files...>`                  | --      | One or more (comma-separated) spec files to load (conflicts with `--specs-dir`) |
| `--specs-dir <dir>`                   | --      | Directory of spec files to load (conflicts with `--specs`)                      |
| `--port <port>`                       | `8532`  | Port to listen on                                                               |
| `--min-items <n>`                     | `1`     | Minimum items in generated arrays                                               |
| `--max-items <n>`                     | `20`    | Maximum items in generated arrays                                               |
| `--use-examples`                      | `false` | Use `example` fields from schemas instead of generating random data             |
| `--cors-allowed-origins <origins...>` | `*`     | Allowed CORS origins; `*` for all, or a comma-separated list                    |

Every flag can also be set via the corresponding environment variable -- see below.

## Environment variables

| Variable                      | Equivalent flag          |
|-------------------------------|--------------------------|
| `HERMIT_SPECS`                | `--specs`                |
| `HERMIT_SPECS_DIR`            | `--specs-dir`            |
| `HERMIT_PORT`                 | `--port`                 |
| `HERMIT_MIN_ITEMS`            | `--min-items`            |
| `HERMIT_MAX_ITEMS`            | `--max-items`            |
| `HERMIT_USE_EXAMPLES`         | `--use-examples`         |
| `HERMIT_CORS_ALLOWED_ORIGINS` | `--cors-allowed-origins` |

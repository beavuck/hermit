# 🦀🐚 Beavuck Hermit

Hermit is a zero-config OpenAPI mock server. Point it at a spec file, and it starts serving schema-accurate responses
immediately -- no stubs to write, no configuration.

## 📊 Status

[![Quality gate](https://sonarcloud.io/api/project_badges/quality_gate?project=beavuck-services_hermit)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

[![Security Rating](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=security_rating)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=reliability_rating)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=bugs)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=code_smells)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)
[![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=sqale_rating)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=sqale_index)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=ncloc)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)
[![Duplicated Lines (%)](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=duplicated_lines_density)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=beavuck-services_hermit&metric=coverage)](https://sonarcloud.io/summary/new_code?id=beavuck-services_hermit)

## 📗 Use cases

Hermit is useful whenever you need an API to be available but don't want to run the real backend:

- **Frontend development** -- build and iterate against real HTTP endpoints without depending on a live backend
- **Integration and E2E testing** -- run tests in CI against a predictable, schema-accurate server with no database or
  external services
- **Contract validation** -- verify that your OpenAPI spec produces the shapes your consumers actually expect

### 🔍 How it works

```mermaid
sequenceDiagram
    participant Client
    participant Hermit
    participant Spec as OpenAPI Spec
    Note over Hermit, Spec: Startup -- once
    Hermit ->> Spec: Read & parse YAML
    Hermit ->> Hermit: Resolve $refs, flatten allOf/oneOf
    Hermit ->> Hermit: Pre-generate mock responses for all routes
    Note over Client, Hermit: Runtime -- per request
    Client ->> Hermit: GET /projects/{projectId}
    Hermit -->> Client: 200 {"id": "abc", "name": "echo", ...}
    Client ->> Hermit: POST /projects {"name": "my project"}
    Hermit ->> Hermit: Merge request body into mock response
    Hermit -->> Client: 201 {"id": "abc", "name": "my project", ...}
```

All schema work (ref resolution, composition, value generation) happens once at startup. Requests are served from an
in-memory map with no I/O.

### 👓 Request body echo

For `POST`, `PUT`, and `PATCH` requests, Hermit merges the fields you send into the mock response. This means your code
sees its own writes reflected back -- the most useful behavior for frontend development:

```bash
# Start Hermit
hermit --specs my-api.openapi.yml

# Create a project -- the response reflects the name you sent
curl -s -X POST http://localhost:8532/projects \
  -H 'Content-Type: application/json' \
  -d '{"name": "Acme Redesign", "status": "active"}' | jq .
# {
#   "id": "foxtrot",
#   "name": "Acme Redesign",
#   "status": "active",
#   ...
# }
```

### 🧬 Polymorphic responses

When a `POST` endpoint uses `oneOf` with a discriminator, Hermit inspects the request body to pick the right response
shape:

```bash
# Creating a "feature" task returns a FeatureTask shape
curl -s -X POST http://localhost:8532/projects/abc/tasks \
  -H 'Content-Type: application/json' \
  -d '{"type": "feature", "storyPoints": 8}' | jq .type
# "feature"

# Creating a "bug" task returns a BugTask shape
curl -s -X POST http://localhost:8532/projects/abc/tasks \
  -H 'Content-Type: application/json' \
  -d '{"type": "bug", "severity": "high"}' | jq .type
# "bug"
```

### 📄 Response generation

Response field values are resolved in priority order:

| Priority | Source              | When                                                                                |
|----------|---------------------|-------------------------------------------------------------------------------------|
| 1        | Request body fields | `POST`, `PUT`, `PATCH` — caller's values always win                                 |
| 2        | `example` in schema | Field has an explicit example value                                                 |
| 3        | Random value        | Fallback — random word, number, boolean, or format-aware value (UUID, date-time, …) |

## 🛠️ Install

```bash
TAG=$(curl -fsSL "https://gitlab.com/api/v4/projects/80082599/releases/permalink/latest" \
  | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4) \
&& sudo curl -fsSL "https://gitlab.com/api/v4/projects/80082599/packages/generic/hermit/${TAG}/hermit" \
  -o /usr/local/bin/hermit \
&& sudo chmod +x /usr/local/bin/hermit
```

Re-run the same command to update to the latest release.

Then run it against your spec (replace the path with your actual spec file):

```bash
hermit --specs ~/Documents/dev/hermit/specs_assets/taskflow.openapi.yml
```

To see what arguments are available, run:

```bash
hermit --help
```

Stop the server with `Ctrl+C`, or if it's running in the background:

```bash
kill $(lsof -ti :8532)
```

## 🔒 Privacy

See [PRIVACY.md](PRIVACY.md).

## 📜 License

See [UNLICENSE](UNLICENSE).

---

_And now some dev stuff_

---

## 🛞 Build and run


```bash
cargo run --release -- --specs specs_assets/taskflow.openapi.yml
```

The server listens on port `8532` by default. Override with `--port`:

```bash
cargo run --release -- --port 9000 --specs specs_assets/taskflow.openapi.yml
```

## 🔧 Development setup

Install `just`:

```bash
cargo install just
```

Install dev tools:

```bash
just setup
```

In RustRover, go to Settings > Version Control > Commit > Advanced commit checks and choose the `pre-commit`
configuration to run these checks automatically on commit.

Explore `justfile` for available commands.

## ✅ API tests

Requires the server to be running. Run with [Bruno](https://www.usebruno.com/):

```bash
cd api_tests/hermit_api_tests && npx --yes @usebruno/cli run --env hermit_env --reporter-html test_report.html --reporter-json test_report.json
```

## 🏗️ Architecture

See [docs/architecture.md](docs/architecture.md).

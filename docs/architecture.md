# Architecture

Hermit is a read-once, serve-many mock server. The specs are parsed at startup, all responses are generated eagerly into
memory, and the server then handles requests purely by lookup -- no parsing or generation happens at request time.

## Module structure

```
src/
  main.rs              Entry point: parses args, wires up the pipeline
  cli.rs               CLI argument definitions (clap)
  constants.rs         Named constants (default port, bind address)
  spec_parser.rs       Loads specs, resolves $refs, flattens schemas, extracts routes
  resource_generator.rs  Generates a serde_json::Value from a resolved schema
  http_method.rs       Classifies which HTTP methods accept a request body
  router.rs            Builds the axum Router; handles requests
```

## Startup flow

Multiple spec files are loaded and parsed in parallel, each on its own OS thread. Within each spec, discriminator
variants are generated in parallel using a Rayon thread pool. Routes from all specs are merged before the server starts.

```mermaid
sequenceDiagram
    participant CLI
    participant spec_parser
    participant resource_generator
    participant router
    participant axum
    CLI ->> spec_parser: load_all(paths)
    par for each spec file (std::thread)
        spec_parser ->> spec_parser: load(path) — read & parse YAML
        spec_parser ->> spec_parser: extract_routes(spec)
        note over spec_parser, resource_generator: For each (path, method) in spec.paths
        spec_parser ->> resource_generator: generate(schema, spec, variant?) — rayon par_iter for discriminator variants
        resource_generator -->> spec_parser: serde_json::Value
    end
    spec_parser -->> CLI: Vec<RouteConfig>
    CLI ->> router: build(routes)
    router -->> CLI: axum::Router
    CLI ->> axum: serve(listener, app)
```

All schema resolution, `$ref` following, and response generation happens during `extract_routes`. By the time the server
is listening, every route has its response pre-built.

## Concurrency model

| Layer | Mechanism | Purpose |
|---|---|---|
| Request handling | Tokio async runtime (multi-threaded) | Concurrent HTTP requests |
| Spec loading | `std::thread::spawn` + `JoinHandle` | Parallel I/O across spec files |
| Discriminator variant generation | `rayon::par_iter` | CPU-parallel value generation |
| Shared state (`CrudStore`) | `Arc<RwLock<...>>` | Multiple readers / exclusive writers |

The `RwLock` on `CrudStore` allows concurrent GET requests to read state simultaneously, while mutating methods (POST,
PUT, PATCH, DELETE) take an exclusive write lock. PATCH acquires a read lock for the initial fetch and a separate write
lock for the store update, so unrelated GETs are not blocked during the read phase.

## Schema resolution

`spec_parser.rs` normalises OpenAPI schemas into a flat `{type, properties}` structure before passing them to the
generator. The resolution order is:

```mermaid
flowchart TD
    A[schema node] --> B{has $ref?}
    B -- yes --> C[resolve_ref -> follow path in root doc]
    C --> A
    B -- no --> D{has allOf?}
    D -- yes --> E[flatten each item, merge all properties\nlater entries win on collision]
    D -- no --> F{has oneOf / anyOf?}
    F -- yes, forced variant --> G[look up variant key in discriminator.mapping\nfall back to index 0]
    F -- yes, no forced variant --> H[pick index 0]
    F -- no --> I[return schema as-is]
```

`flatten_schema` is the unforced entry point (used for GET responses). `flatten_schema_forced(variant)` is used when
pre-generating each discriminator variant for POST/PUT/PATCH routes.

## Response generation

`resource_generator.rs` walks a flattened schema and produces a `serde_json::Value`. Priority order:

| Schema                        | Output                                                |
|-------------------------------|-------------------------------------------------------|
| has `example`                 | the example value, verbatim                           |
| has `enum`                    | random pick from the enum values                      |
| `type: object`                | object with each property recursively generated       |
| `type: array` with `items`    | single-element array from items schema                |
| `type: array` without `items` | empty array                                           |
| `type: string` with `format`  | format-aware random value (UUID, date-time, email, …) |
| `type: string`                | random word                                           |
| `type: integer` / `number`    | random integer in 1–1000                              |
| `type: boolean`               | random true/false                                     |
| anything else                 | `null`                                                |

## Request handling

```mermaid
flowchart LR
    req[HTTP request] --> match[match method]
    match -- GET / DELETE\nOPTIONS / HEAD / TRACE --> ro[handle_readonly]
    match -- POST / PUT / PATCH --> wb[handle_with_body]
    ro --> lookup[(AppState\nroute map)]
    lookup -- 204 route --> 204[204 No Content]
    lookup -- body route --> json[JSON response]
    lookup -- not found --> 404[404]
    wb --> lookup
    wb --> body[parse request body]
    body --> disc{discriminator\nfield?}
    disc -- yes --> variant[pick pre-generated variant\nby discriminator value]
    disc -- no --> base[use pre-generated body]
    variant --> merge[merge: overlay request\nfields onto base]
    base --> merge
    merge --> json
```

The route lookup key is `"METHOD /path/{param}"`. For body-accepting methods, the request body is merged on top of the
pre-generated response -- request fields win over spec-derived fields.

## Discriminator variants

When a POST/PUT/PATCH response schema uses `oneOf`/`anyOf` with an OpenAPI `discriminator`, Hermit pre-generates one
response body per mapping key at startup. At request time it reads the discriminator field from the request body and
serves the matching variant. If the value is absent or unrecognized, it falls back to the first variant.

This means polymorphic APIs (e.g. a task endpoint that returns `FeatureTask | BugTask | ChoreTask` depending on the
`type` field) work correctly without any per-endpoint configuration.

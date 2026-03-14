use std::collections::HashMap;
use std::sync::Arc;
use yaml_serde::Value;

use rayon::prelude::*;

use crate::http_method::HttpMethod;

pub fn load(path: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(path).expect("spec file not found");
    yaml_serde::from_str(&text).expect("invalid YAML")
}

pub fn load_all(paths: &[std::path::PathBuf]) -> Vec<RouteConfig> {
    let handles: Vec<_> = paths
        .iter()
        .map(|path| {
            let path = path.clone();
            std::thread::spawn(move || {
                let spec = load(&path);
                extract_routes(&spec)
            })
        })
        .collect();

    handles
        .into_iter()
        .flat_map(|h| h.join().expect("spec loader thread panicked"))
        .collect()
}

pub fn resolve_ref<'a>(root: &'a Value, ref_str: &str) -> Option<&'a Value> {
    let path = ref_str.trim_start_matches('#').trim_start_matches('/');
    let mut current = root;
    for key in path.split('/') {
        current = current.get(key)?;
    }
    Some(current)
}

pub fn flatten_schema(schema: &Value, root: &Value) -> Value {
    flatten_inner(schema, root, None)
}

pub fn flatten_schema_forced(schema: &Value, root: &Value, variant: &str) -> Value {
    flatten_inner(schema, root, Some(variant))
}

fn flatten_inner(schema: &Value, root: &Value, forced: Option<&str>) -> Value {
    if schema.get("$ref").is_some() {
        return flatten_ref(schema, root, forced);
    }
    if schema.get("allOf").is_some() {
        return flatten_all_of(schema, root, forced);
    }
    try_flatten_composite(schema, root, forced).unwrap_or_else(|| schema.clone())
}

fn flatten_ref(schema: &Value, root: &Value, forced: Option<&str>) -> Value {
    schema
        .get("$ref")
        .and_then(|v| v.as_str())
        .and_then(|ref_str| resolve_ref(root, ref_str))
        .map(|resolved| flatten_inner(resolved, root, forced))
        .unwrap_or_else(|| schema.clone())
}

fn flatten_all_of(schema: &Value, root: &Value, forced: Option<&str>) -> Value {
    let merged = schema
        .get("allOf")
        .and_then(|v| v.as_sequence())
        .map(|items| merge_all_of_properties(items, root, forced))
        .unwrap_or_default();
    let mut result = yaml_serde::Mapping::new();
    result.insert(Value::String("type".into()), Value::String("object".into()));
    result.insert(Value::String("properties".into()), Value::Mapping(merged));
    Value::Mapping(result)
}

fn merge_all_of_properties(
    items: &[Value],
    root: &Value,
    forced: Option<&str>,
) -> yaml_serde::Mapping {
    let mut merged = yaml_serde::Mapping::new();
    for item in items {
        let flat = flatten_inner(item, root, forced);
        if let Some(props) = flat.get("properties").and_then(|v| v.as_mapping()) {
            for (k, v) in props {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    merged
}

fn try_flatten_composite(schema: &Value, root: &Value, forced: Option<&str>) -> Option<Value> {
    let variants = ["oneOf", "anyOf"].iter().find_map(|key| {
        let seq = schema.get(*key)?.as_sequence()?;
        if seq.is_empty() { None } else { Some(seq) }
    })?;

    if let Some(forced_key) = forced
        && let Some(result) = try_forced_variant(schema, root, forced_key, forced)
    {
        return Some(result);
    }
    Some(flatten_inner(&variants[0], root, forced))
}

fn try_forced_variant(
    schema: &Value,
    root: &Value,
    forced_key: &str,
    forced: Option<&str>,
) -> Option<Value> {
    let mapping = schema.get("discriminator")?.get("mapping")?.as_mapping()?;
    let lookup = Value::String(forced_key.to_string());
    let ref_str = mapping.get(&lookup)?.as_str()?;
    let resolved = resolve_ref(root, ref_str)?;
    Some(flatten_inner(resolved, root, forced))
}

pub fn find_discriminator(schema: &Value, root: &Value) -> Option<(String, Vec<String>)> {
    if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
        return resolve_ref(root, ref_str).and_then(|resolved| find_discriminator(resolved, root));
    }

    for composite_key in &["oneOf", "anyOf"] {
        if schema.get(*composite_key).is_some()
            && let Some(info) = discriminator_info(schema)
        {
            return Some(info);
        }
    }

    schema
        .get("allOf")
        .and_then(|v| v.as_sequence())
        .and_then(|items| items.iter().find_map(|item| find_discriminator(item, root)))
}

fn discriminator_info(schema: &Value) -> Option<(String, Vec<String>)> {
    let disc = schema.get("discriminator")?;
    let prop_name = disc.get("propertyName")?.as_str()?;
    let keys = disc
        .get("mapping")
        .and_then(|m| m.as_mapping())
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Some((prop_name.to_string(), keys))
}

#[derive(Clone)]
pub struct RouteConfig {
    pub axum_path: String,
    pub method: HttpMethod,
    pub status_code: u16,
    pub body: Option<serde_json::Value>,
    pub discriminator_field: Option<String>,
    pub variants: Option<HashMap<String, serde_json::Value>>,
    pub item_generator: Option<Arc<dyn Fn() -> serde_json::Value + Send + Sync>>,
    pub read_only_fields: std::collections::HashSet<String>,
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            axum_path: String::new(),
            method: HttpMethod::Get,
            status_code: 200,
            body: None,
            discriminator_field: None,
            variants: None,
            item_generator: None,
            read_only_fields: Default::default(),
        }
    }
}

pub fn extract_routes(spec: &Value) -> Vec<RouteConfig> {
    let spec_arc = Arc::new(spec.clone());
    let base = server_base_path(spec);
    let Some(paths) = spec.get("paths").and_then(|v| v.as_mapping()) else {
        return Vec::new();
    };

    let mut routes = Vec::new();
    for (path_val, path_item) in paths {
        let Some(path_str) = path_val.as_str() else {
            continue;
        };
        let axum_path = format!("{}{}", base, path_str);
        for &method in HttpMethod::ALL {
            if let Some(mut route) = path_item
                .get(method.as_str())
                .and_then(|op| route_for_operation(path_str, method, op, spec, &spec_arc))
            {
                route.axum_path = axum_path.clone();
                routes.push(route);
            }
        }
    }
    routes
}

fn server_base_path(spec: &Value) -> String {
    let url = spec
        .get("servers")
        .and_then(|s| s.get(0))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or("");
    let path = if url.contains("://") {
        url.splitn(4, '/')
            .nth(3)
            .map(|p| format!("/{}", p))
            .unwrap_or_default()
    } else {
        url.to_string()
    };
    let trimmed = path.trim_end_matches('/');
    if trimmed == "/" || trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn route_for_operation(
    path: &str,
    method: HttpMethod,
    operation: &Value,
    spec: &Value,
    spec_arc: &Arc<Value>,
) -> Option<RouteConfig> {
    let responses = operation.get("responses")?;
    let (status_code, response) = first_success_response(responses);

    let schema = if status_code != NO_BODY_STATUS {
        extract_response_schema(response, spec)
    } else {
        None
    };

    Some(match schema {
        Some(s) => route_from_schema(path, method, status_code, s, spec, spec_arc),
        None => RouteConfig {
            axum_path: path.to_string(),
            method,
            status_code,
            ..Default::default()
        },
    })
}

fn route_from_schema(
    path: &str,
    method: HttpMethod,
    status_code: u16,
    schema: &Value,
    spec: &Value,
    spec_arc: &Arc<Value>,
) -> RouteConfig {
    let read_only_fields = collect_read_only_fields(schema, spec);
    if method.uses_request_body()
        && let Some((disc_field, keys)) = find_discriminator(schema, spec)
    {
        let variants = keys
            .par_iter()
            .map(|key| {
                (
                    key.clone(),
                    crate::resource_generator::generate(schema, spec, Some(key)),
                )
            })
            .collect();
        return RouteConfig {
            axum_path: path.to_string(),
            method,
            status_code,
            discriminator_field: Some(disc_field),
            variants: Some(variants),
            read_only_fields,
            ..Default::default()
        };
    }
    let item_generator = build_item_generator(schema, spec, spec_arc);
    RouteConfig {
        axum_path: path.to_string(),
        method,
        status_code,
        body: Some(crate::resource_generator::generate(schema, spec, None)),
        item_generator,
        read_only_fields,
        ..Default::default()
    }
}

fn collect_read_only_fields(schema: &Value, root: &Value) -> std::collections::HashSet<String> {
    let flat = flatten_schema(schema, root);
    let mut fields = std::collections::HashSet::new();
    if let Some(props) = flat.get("properties").and_then(|v| v.as_mapping()) {
        for (k, v) in props {
            if let Some(key) = k.as_str()
                && v.get("readOnly").and_then(|b| b.as_bool()).unwrap_or(false)
            {
                fields.insert(key.to_string());
            }
        }
    }
    fields
}

fn build_item_generator(
    response_schema: &Value,
    root: &Value,
    spec_arc: &Arc<Value>,
) -> Option<Arc<dyn Fn() -> serde_json::Value + Send + Sync>> {
    let item_schema = extract_item_schema(response_schema, root)?;
    let spec_for_gen = Arc::clone(spec_arc);
    Some(Arc::new(move || {
        crate::resource_generator::generate(&item_schema, &spec_for_gen, None)
    }))
}

fn extract_item_schema(response_schema: &Value, root: &Value) -> Option<Value> {
    let flat = flatten_schema(response_schema, root);
    // Direct array response
    if flat.get("type").and_then(|t| t.as_str()) == Some("array") {
        return flat.get("items").cloned();
    }
    // Envelope object with an array property
    flat.get("properties")
        .and_then(|p| p.as_mapping())
        .and_then(|m| {
            m.values()
                .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("array"))
        })
        .and_then(|arr_schema| arr_schema.get("items"))
        .cloned()
}

const NO_BODY_STATUS: u16 = 204;
const SUCCESS_STATUS_CODES: &[u16] = &[200, 201, 204];

fn first_success_response(responses: &Value) -> (u16, &Value) {
    for &code in SUCCESS_STATUS_CODES {
        if let Some(resp) = responses.get(code.to_string().as_str()) {
            return (code, resp);
        }
    }
    (200, responses)
}

fn extract_response_schema<'a>(response: &'a Value, root: &'a Value) -> Option<&'a Value> {
    let resp = if let Some(ref_str) = response.get("$ref").and_then(|v| v.as_str()) {
        resolve_ref(root, ref_str)?
    } else {
        response
    };

    resp.get("content")
        .and_then(|c| c.get("application/json"))
        .and_then(|j| j.get("schema"))
}

#[cfg(test)]
mod tests {
    use crate::http_method::HttpMethod;

    use super::*;

    fn yaml(s: &str) -> Value {
        yaml_serde::from_str(s).unwrap()
    }

    // --- resolve_ref ---

    #[test]
    fn resolve_ref_finds_nested_value() {
        let root = yaml("a:\n  b:\n    c: found");
        let result = resolve_ref(&root, "#/a/b/c");
        assert_eq!(result.and_then(|v| v.as_str()), Some("found"));
    }

    #[test]
    fn resolve_ref_returns_none_for_missing_key() {
        let root = yaml("a: 1");
        assert!(resolve_ref(&root, "#/missing").is_none());
    }

    #[test]
    fn resolve_ref_returns_none_for_missing_nested_key() {
        let root = yaml("a:\n  b: 1");
        assert!(resolve_ref(&root, "#/a/missing").is_none());
    }

    // --- flatten_schema ---

    #[test]
    fn flatten_schema_returns_plain_schema_unchanged() {
        let root = yaml("{}");
        let schema = yaml("type: string\nexample: hello");
        let flat = flatten_schema(&schema, &root);
        assert_eq!(flat.get("type").and_then(|v| v.as_str()), Some("string"));
        assert_eq!(flat.get("example").and_then(|v| v.as_str()), Some("hello"));
    }

    #[test]
    fn flatten_schema_follows_ref() {
        let root = yaml("Foo:\n  type: string\n  example: hi");
        let schema = yaml("$ref: '#/Foo'");
        let flat = flatten_schema(&schema, &root);
        assert_eq!(flat.get("type").and_then(|v| v.as_str()), Some("string"));
    }

    #[test]
    fn flatten_schema_unresolvable_ref_returns_schema_unchanged() {
        let root = yaml("{}");
        let schema = yaml("$ref: '#/Missing'");
        let flat = flatten_schema(&schema, &root);
        assert_eq!(flat.get("$ref").and_then(|v| v.as_str()), Some("#/Missing"));
    }

    #[test]
    fn flatten_schema_merges_all_of_properties() {
        let root = yaml("{}");
        let schema = yaml(
            "allOf:\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     a:\n\
             \x20       type: string\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     b:\n\
             \x20       type: integer",
        );
        let flat = flatten_schema(&schema, &root);
        assert!(flat.get("properties").and_then(|p| p.get("a")).is_some());
        assert!(flat.get("properties").and_then(|p| p.get("b")).is_some());
    }

    #[test]
    fn flatten_schema_all_of_later_entry_wins_on_collision() {
        let root = yaml("{}");
        let schema = yaml(
            "allOf:\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     x:\n\
             \x20       example: first\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     x:\n\
             \x20       example: second",
        );
        let flat = flatten_schema(&schema, &root);
        let example = flat
            .get("properties")
            .and_then(|p| p.get("x"))
            .and_then(|x| x.get("example"))
            .and_then(|e| e.as_str());
        assert_eq!(example, Some("second"));
    }

    #[test]
    fn flatten_schema_picks_first_one_of_by_default() {
        let root = yaml("{}");
        let schema = yaml(
            "oneOf:\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     k:\n\
             \x20       example: first\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     k:\n\
             \x20       example: second",
        );
        let flat = flatten_schema(&schema, &root);
        let example = get_example(&flat);

        assert_eq!(example, Some("first"));
    }

    #[test]
    fn flatten_schema_picks_first_any_of_by_default() {
        let root = yaml("{}");
        let schema = yaml(
            "anyOf:\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     k:\n\
             \x20       example: first\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     k:\n\
             \x20       example: second",
        );
        let flat = flatten_schema(&schema, &root);
        let example = get_example(&flat);

        assert_eq!(example, Some("first"));
    }

    #[test]
    fn flatten_schema_forced_picks_mapped_variant() {
        let root = yaml(
            "A:\n  type: object\n  properties:\n    k:\n      example: a\n\
             B:\n  type: object\n  properties:\n    k:\n      example: b",
        );
        let schema = yaml(
            "oneOf:\n\
             \x20 - $ref: '#/A'\n\
             \x20 - $ref: '#/B'\n\
             discriminator:\n\
             \x20 propertyName: k\n\
             \x20 mapping:\n\
             \x20   a: '#/A'\n\
             \x20   b: '#/B'",
        );
        let flat = flatten_schema_forced(&schema, &root, "b");
        let example = get_example(&flat);

        assert_eq!(example, Some("b"));
    }

    #[test]
    fn flatten_schema_forced_falls_back_to_first_when_key_not_in_mapping() {
        let root = yaml(
            "A:\n  type: object\n  properties:\n    k:\n      example: a\n\
             B:\n  type: object\n  properties:\n    k:\n      example: b",
        );
        let schema = yaml(
            "oneOf:\n\
             \x20 - $ref: '#/A'\n\
             \x20 - $ref: '#/B'\n\
             discriminator:\n\
             \x20 propertyName: k\n\
             \x20 mapping:\n\
             \x20   a: '#/A'\n\
             \x20   b: '#/B'",
        );
        let flat = flatten_schema_forced(&schema, &root, "unknown");
        let example = get_example(&flat);

        assert_eq!(example, Some("a"));
    }

    fn get_example(flat: &Value) -> Option<&str> {
        flat.get("properties")
            .and_then(|p| p.get("k"))
            .and_then(|k| k.get("example"))
            .and_then(|e| e.as_str())
    }
    // --- find_discriminator ---

    #[test]
    fn find_discriminator_finds_direct_one_of_discriminator() {
        let root = yaml("{}");
        let schema = yaml(
            "oneOf:\n\
             \x20 - type: object\n\
             discriminator:\n\
             \x20 propertyName: type\n\
             \x20 mapping:\n\
             \x20   a: '#/a'\n\
             \x20   b: '#/b'",
        );
        let (field, keys) = find_discriminator(&schema, &root).unwrap();
        assert_eq!(field, "type");
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
    }

    #[test]
    fn find_discriminator_finds_any_of_discriminator() {
        let root = yaml("{}");
        let schema = yaml(
            "anyOf:\n\
             \x20 - type: object\n\
             discriminator:\n\
             \x20 propertyName: kind\n\
             \x20 mapping:\n\
             \x20   x: '#/x'",
        );
        let (field, _) = find_discriminator(&schema, &root).unwrap();
        assert_eq!(field, "kind");
    }

    #[test]
    fn find_discriminator_finds_discriminator_inside_all_of() {
        let root = yaml("{}");
        let schema = yaml(
            "allOf:\n\
             \x20 - oneOf:\n\
             \x20     - type: object\n\
             \x20   discriminator:\n\
             \x20     propertyName: kind\n\
             \x20     mapping:\n\
             \x20       x: '#/x'",
        );
        let (field, _) = find_discriminator(&schema, &root).unwrap();
        assert_eq!(field, "kind");
    }

    #[test]
    fn find_discriminator_follows_ref() {
        let root = yaml(
            "Poly:\n\
             \x20 oneOf:\n\
             \x20   - type: object\n\
             \x20 discriminator:\n\
             \x20   propertyName: type\n\
             \x20   mapping:\n\
             \x20     a: '#/a'",
        );
        let schema = yaml("$ref: '#/Poly'");
        let (field, _) = find_discriminator(&schema, &root).unwrap();
        assert_eq!(field, "type");
    }

    #[test]
    fn find_discriminator_returns_none_when_absent() {
        let root = yaml("{}");
        let schema = yaml("type: object\nproperties:\n  x:\n    type: string");
        assert!(find_discriminator(&schema, &root).is_none());
    }

    // --- extract_routes ---

    #[test]
    fn extract_routes_returns_empty_for_spec_without_paths() {
        let spec = yaml("info:\n  title: Test");
        assert!(extract_routes(&spec).is_empty());
    }

    #[test]
    fn extract_routes_delete_with_204_has_no_body() {
        let spec = yaml(
            "paths:\n\
             \x20 /items/{id}:\n\
             \x20   delete:\n\
             \x20     responses:\n\
             \x20       '204':\n\
             \x20         description: Deleted",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].status_code, 204);
        assert!(routes[0].body.is_none());
        assert!(routes[0].variants.is_none());
    }

    #[test]
    fn extract_routes_get_generates_body_from_schema() {
        let spec = yaml(
            "paths:\n\
             \x20 /items:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         content:\n\
             \x20           application/json:\n\
             \x20             schema:\n\
             \x20               type: object\n\
             \x20               properties:\n\
             \x20                 name:\n\
             \x20                   type: string\n\
             \x20                   example: hello\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].method, HttpMethod::Get);
        assert_eq!(routes[0].status_code, 200);
        assert_eq!(
            routes[0].body.as_ref().unwrap()["name"],
            serde_json::json!("hello")
        );
    }

    #[test]
    fn extract_routes_response_without_schema_has_no_body() {
        let spec = yaml(
            "paths:\n\
             \x20 /items:\n\
             \x20   options:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert!(routes[0].body.is_none());
    }

    #[test]
    fn extract_routes_post_with_discriminator_generates_variants() {
        let spec = yaml(
            "components:\n\
             \x20 schemas:\n\
             \x20   A:\n\
             \x20     type: object\n\
             \x20     properties:\n\
             \x20       k: {type: string, example: a}\n\
             \x20   B:\n\
             \x20     type: object\n\
             \x20     properties:\n\
             \x20       k: {type: string, example: b}\n\
             paths:\n\
             \x20 /items:\n\
             \x20   post:\n\
             \x20     responses:\n\
             \x20       '201':\n\
             \x20         content:\n\
             \x20           application/json:\n\
             \x20             schema:\n\
             \x20               oneOf:\n\
             \x20                 - $ref: '#/components/schemas/A'\n\
             \x20                 - $ref: '#/components/schemas/B'\n\
             \x20               discriminator:\n\
             \x20                 propertyName: k\n\
             \x20                 mapping:\n\
             \x20                   a: '#/components/schemas/A'\n\
             \x20                   b: '#/components/schemas/B'\n\
             \x20         description: Created",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].discriminator_field.as_deref(), Some("k"));
        let variants = routes[0].variants.as_ref().unwrap();
        assert_eq!(variants["a"]["k"], serde_json::json!("a"));
        assert_eq!(variants["b"]["k"], serde_json::json!("b"));
        assert!(routes[0].body.is_none());
    }

    #[test]
    fn load_reads_yaml_from_file() {
        let val = load(std::path::Path::new("specs_assets/taskflow.openapi.yml"));
        assert!(val.get("paths").is_some());
    }

    #[test]
    fn load_all_returns_routes_from_all_specs_in_parallel() {
        let path = std::path::PathBuf::from("specs_assets/taskflow.openapi.yml");
        let routes = load_all(&[path.clone(), path]);
        assert!(routes.len() >= 2, "expected routes from both specs");
    }

    #[test]
    fn flatten_schema_all_of_item_without_properties_is_skipped() {
        let root = yaml("{}");
        let schema = yaml(
            "allOf:\n\
             \x20 - type: string\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     x:\n\
             \x20       type: string\n\
             \x20       example: kept",
        );
        let flat = flatten_schema(&schema, &root);
        assert!(flat.get("properties").and_then(|p| p.get("x")).is_some());
    }

    #[test]
    fn extract_routes_ignores_non_string_path_keys() {
        let spec = yaml(
            "paths:\n\
             \x20 123:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        assert!(extract_routes(&spec).is_empty());
    }

    #[test]
    fn extract_routes_falls_back_to_200_when_no_success_code() {
        let spec = yaml(
            "paths:\n\
             \x20 /items:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '500':\n\
             \x20         description: Error",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].status_code, 200);
        assert!(routes[0].body.is_none());
    }

    #[test]
    fn extract_routes_resolves_ref_response() {
        let spec = yaml(
            "components:\n\
             \x20 responses:\n\
             \x20   MyResponse:\n\
             \x20     content:\n\
             \x20       application/json:\n\
             \x20         schema:\n\
             \x20           type: object\n\
             \x20           properties:\n\
             \x20             id:\n\
             \x20               type: string\n\
             \x20               example: abc\n\
             paths:\n\
             \x20 /items:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         $ref: '#/components/responses/MyResponse'",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes.len(), 1);
        assert_eq!(
            routes[0].body.as_ref().unwrap()["id"],
            serde_json::json!("abc")
        );
    }

    // --- server base path ---

    #[test]
    fn extract_routes_prefixes_paths_with_server_base_path() {
        let spec = yaml(
            "servers:\n\
             \x20 - url: /api/dog-cafe\n\
             paths:\n\
             \x20 /dogs:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/api/dog-cafe/dogs");
    }

    #[test]
    fn extract_routes_extracts_path_from_full_server_url() {
        let spec = yaml(
            "servers:\n\
             \x20 - url: http://127.0.0.1:8532/api/dog-cafe\n\
             paths:\n\
             \x20 /dogs:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/api/dog-cafe/dogs");
    }

    #[test]
    fn extract_routes_with_root_server_url_does_not_prefix() {
        let spec = yaml(
            "servers:\n\
             \x20 - url: /\n\
             paths:\n\
             \x20 /dogs:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/dogs");
    }

    #[test]
    fn extract_routes_with_no_servers_does_not_prefix() {
        let spec = yaml(
            "paths:\n\
             \x20 /dogs:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/dogs");
    }

    #[test]
    fn extract_routes_preserves_path_parameters_after_prefix() {
        let spec = yaml(
            "servers:\n\
             \x20 - url: /api/v1\n\
             paths:\n\
             \x20 /dogs/{id}:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/api/v1/dogs/{id}");
    }

    #[test]
    fn extract_routes_preserves_openapi_path_format() {
        let spec = yaml(
            "paths:\n\
             \x20 /projects/{projectId}:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        assert_eq!(routes[0].axum_path, "/projects/{projectId}");
    }

    // --- read_only_fields collection ---

    #[test]
    fn extract_routes_post_collects_read_only_fields_from_response_schema() {
        let spec = yaml(
            "paths:\n\
             \x20 /items:\n\
             \x20   post:\n\
             \x20     responses:\n\
             \x20       '201':\n\
             \x20         content:\n\
             \x20           application/json:\n\
             \x20             schema:\n\
             \x20               type: object\n\
             \x20               properties:\n\
             \x20                 id:\n\
             \x20                   type: string\n\
             \x20                   readOnly: true\n\
             \x20                 name:\n\
             \x20                   type: string\n\
             \x20         description: Created",
        );
        let routes = extract_routes(&spec);
        assert!(routes[0].read_only_fields.contains("id"));
        assert!(!routes[0].read_only_fields.contains("name"));
    }

    // --- item_generator invocation ---

    #[test]
    fn extract_routes_item_generator_produces_a_value_matching_schema() {
        let spec = yaml(
            "paths:\n\
             \x20 /items:\n\
             \x20   get:\n\
             \x20     responses:\n\
             \x20       '200':\n\
             \x20         content:\n\
             \x20           application/json:\n\
             \x20             schema:\n\
             \x20               type: array\n\
             \x20               items:\n\
             \x20                 type: object\n\
             \x20                 properties:\n\
             \x20                   name:\n\
             \x20                     type: string\n\
             \x20                     example: hello\n\
             \x20         description: OK",
        );
        let routes = extract_routes(&spec);
        let r#gen = routes[0]
            .item_generator
            .as_ref()
            .expect("expected item_generator");
        let value = r#gen();
        assert!(value.is_object());
    }
}

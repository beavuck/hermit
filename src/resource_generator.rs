use rand::RngExt;
use serde_json::Value as JsonValue;
use yaml_serde::Value as YamlValue;

use crate::spec_parser::{flatten_schema, flatten_schema_forced};

pub fn generate(schema: &YamlValue, root: &YamlValue, forced_variant: Option<&str>) -> JsonValue {
    let flat = match forced_variant {
        Some(v) => flatten_schema_forced(schema, root, v),
        None => flatten_schema(schema, root),
    };
    generate_flat(&flat, root, forced_variant)
}

fn generate_flat(flat: &YamlValue, root: &YamlValue, forced: Option<&str>) -> JsonValue {
    if let Some(example) = flat.get("example") {
        return yaml_to_json(example);
    }

    if let Some(enum_seq) = flat.get("enum").and_then(|v| v.as_sequence())
        && !enum_seq.is_empty()
    {
        let idx = rand::rng().random_range(0..enum_seq.len());
        return yaml_to_json(&enum_seq[idx]);
    }

    match flat.get("type").and_then(|v| v.as_str()).unwrap_or("") {
        "object" => generate_object(flat, root, forced),
        "array" => generate_array(flat, root, forced),
        t => primitive_fallback(flat, t),
    }
}

fn generate_object(schema: &YamlValue, root: &YamlValue, forced: Option<&str>) -> JsonValue {
    let mut map = serde_json::Map::new();
    if let Some(props) = schema.get("properties").and_then(|v| v.as_mapping()) {
        for (k, v) in props {
            if let Some(key) = k.as_str() {
                map.insert(key.to_string(), generate(v, root, forced));
            }
        }
    }
    JsonValue::Object(map)
}

fn generate_array(schema: &YamlValue, root: &YamlValue, forced: Option<&str>) -> JsonValue {
    match schema.get("items") {
        Some(items_schema) => JsonValue::Array(vec![generate(items_schema, root, forced)]),
        None => JsonValue::Array(vec![]),
    }
}

const RANDOM_WORDS: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
];

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn primitive_fallback(schema: &YamlValue, schema_type: &str) -> JsonValue {
    let mut rng = rand::rng();
    match schema_type {
        "string" => {
            let fmt = schema.get("format").and_then(|v| v.as_str()).unwrap_or("");
            let s = if fmt.is_empty() {
                let word_count = rng.random_range(2..=5usize);
                (0..word_count)
                    .map(|_| random_word(&mut rng))
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                string_for_format(fmt, &mut rng)
            };
            JsonValue::String(s)
        }
        "integer" | "number" => JsonValue::Number(rng.random_range(1i64..=1000).into()),
        "boolean" => JsonValue::Bool(rng.random()),
        _ => JsonValue::Null,
    }
}

fn random_word(rng: &mut impl RngExt) -> &'static str {
    RANDOM_WORDS[rng.random_range(0..RANDOM_WORDS.len())]
}

fn string_for_format(fmt: &str, rng: &mut impl RngExt) -> String {
    match fmt {
        "date-time" => format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            rng.random_range(2000u16..=2030),
            rng.random_range(1u8..=12),
            rng.random_range(1u8..=28),
            rng.random_range(0u8..=23),
            rng.random_range(0u8..=59),
            rng.random_range(0u8..=59),
        ),
        "date" => format!(
            "{:04}-{:02}-{:02}",
            rng.random_range(2000u16..=2030),
            rng.random_range(1u8..=12),
            rng.random_range(1u8..=28),
        ),
        "time" => format!(
            "{:02}:{:02}:{:02}Z",
            rng.random_range(0u8..=23),
            rng.random_range(0u8..=59),
            rng.random_range(0u8..=59),
        ),
        "uuid" => format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rng.random::<u32>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u64>() & 0x0000_ffff_ffff_ffff,
        ),
        "email" => format!("{}@{}.com", random_word(rng), random_word(rng)),
        "uri" => format!("https://{}.com/{}", random_word(rng), random_word(rng)),
        "hostname" => format!("{}.{}", random_word(rng), random_word(rng)),
        "ipv4" => format!(
            "{}.{}.{}.{}",
            rng.random_range(1u8..=254),
            rng.random::<u8>(),
            rng.random::<u8>(),
            rng.random_range(1u8..=254),
        ),
        "ipv6" => format!(
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
        ),
        "byte" => (0..8)
            .map(|_| BASE64_CHARS[rng.random_range(0..64)] as char)
            .collect(),
        "password" => format!("{}-{}", random_word(rng), random_word(rng)),
        _ => String::new(),
    }
}

fn yaml_to_json(v: &YamlValue) -> JsonValue {
    serde_json::to_value(v).unwrap_or(JsonValue::Null)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::generate;

    fn yaml(s: &str) -> yaml_serde::Value {
        yaml_serde::from_str(s).unwrap()
    }

    // --- example takes priority ---

    #[test]
    fn generate_returns_top_level_example() {
        let root = yaml("{}");
        let schema = yaml("type: string\nexample: hello");
        assert_eq!(generate(&schema, &root, None), json!("hello"));
    }

    // --- object generation ---

    #[test]
    fn generate_object_includes_all_properties() {
        let root = yaml("{}");
        let schema = yaml(
            "type: object\n\
             properties:\n\
             \x20 name:\n\
             \x20   type: string\n\
             \x20   example: Alice\n\
             \x20 age:\n\
             \x20   type: integer\n\
             \x20   example: 30",
        );
        let result = generate(&schema, &root, None);
        assert_eq!(result["name"], json!("Alice"));
        assert_eq!(result["age"], json!(30));
    }

    #[test]
    fn generate_object_with_no_properties_returns_empty_object() {
        let root = yaml("{}");
        let schema = yaml("type: object");
        assert_eq!(generate(&schema, &root, None), json!({}));
    }

    // --- array generation ---

    #[test]
    fn generate_array_returns_single_item_from_items_schema() {
        let root = yaml("{}");
        let schema = yaml("type: array\nitems:\n  type: string\n  example: item");
        let result = generate(&schema, &root, None);
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 1);
        assert_eq!(result[0], json!("item"));
    }

    #[test]
    fn generate_array_with_example_returns_example() {
        let root = yaml("{}");
        let schema = yaml("type: array\nexample: [a, b]\nitems:\n  type: string");
        assert_eq!(generate(&schema, &root, None), json!(["a", "b"]));
    }

    #[test]
    fn generate_array_without_items_returns_empty_array() {
        let root = yaml("{}");
        let schema = yaml("type: array");
        assert_eq!(generate(&schema, &root, None), json!([]));
    }

    // --- primitive defaults ---

    #[test]
    fn generate_string_without_example_returns_a_non_empty_string() {
        let root = yaml("{}");
        let result = generate(&yaml("type: string"), &root, None);
        assert!(result.as_str().map(|s| !s.is_empty()).unwrap_or(false));
    }

    #[test]
    fn generate_string_without_example_is_a_multi_word_phrase() {
        let root = yaml("{}");
        let result = generate(&yaml("type: string"), &root, None);
        let s = result.as_str().expect("expected a string");
        assert!(s.contains(' '), "expected a multi-word phrase, got {s:?}");
    }

    #[test]
    fn generate_integer_without_example_returns_a_positive_integer() {
        let root = yaml("{}");
        let result = generate(&yaml("type: integer"), &root, None);
        assert!(result.as_i64().map(|n| n > 0).unwrap_or(false));
    }

    #[test]
    fn generate_number_without_example_returns_a_positive_number() {
        let root = yaml("{}");
        let result = generate(&yaml("type: number"), &root, None);
        assert!(result.as_f64().map(|n| n > 0.0).unwrap_or(false));
    }

    #[test]
    fn generate_boolean_without_example_returns_a_boolean() {
        let root = yaml("{}");
        let result = generate(&yaml("type: boolean"), &root, None);
        assert!(result.is_boolean());
    }

    #[test]
    fn generate_unknown_type_returns_null() {
        let root = yaml("{}");
        assert_eq!(generate(&yaml("type: unknown"), &root, None), json!(null));
    }

    // --- enums ---

    #[test]
    fn generate_string_enum_returns_one_of_the_enum_values() {
        let root = yaml("{}");
        let schema = yaml("type: string\nenum: [foo, bar, baz]");
        let result = generate(&schema, &root, None);
        assert!(["foo", "bar", "baz"].contains(&result.as_str().unwrap()));
    }

    #[test]
    fn generate_integer_enum_returns_one_of_the_enum_values() {
        let root = yaml("{}");
        let schema = yaml("type: integer\nenum: [1, 2, 3]");
        let result = generate(&schema, &root, None);
        assert!([1, 2, 3].contains(&(result.as_i64().unwrap() as i32)));
    }

    #[test]
    fn generate_single_value_enum_always_returns_that_value() {
        let root = yaml("{}");
        let schema = yaml("type: string\nenum: [only]");
        assert_eq!(generate(&schema, &root, None), json!("only"));
    }

    // --- formats ---

    fn assert_matches(value: &serde_json::Value, pattern: &str) {
        let s = value.as_str().expect("expected a string value");
        let re = regex::Regex::new(pattern).unwrap();
        assert!(
            re.is_match(s),
            "value {s:?} did not match pattern {pattern:?}"
        );
    }

    #[test]
    fn generate_date_time_format_matches_iso8601() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: date-time"), &root, None),
            r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}",
        );
    }

    #[test]
    fn generate_date_format_matches_iso8601_date() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: date"), &root, None),
            r"^\d{4}-\d{2}-\d{2}$",
        );
    }

    #[test]
    fn generate_time_format_matches_hhmmss() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: time"), &root, None),
            r"^\d{2}:\d{2}:\d{2}",
        );
    }

    #[test]
    fn generate_uuid_format_matches_uuid_pattern() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: uuid"), &root, None),
            r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
        );
    }

    #[test]
    fn generate_email_format_matches_email_pattern() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: email"), &root, None),
            r"^[^@\s]+@[^@\s]+\.[^@\s]+$",
        );
    }

    #[test]
    fn generate_uri_format_matches_http_url() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: uri"), &root, None),
            r"^https?://",
        );
    }

    #[test]
    fn generate_hostname_format_matches_hostname_pattern() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: hostname"), &root, None),
            r"^[a-zA-Z0-9]([a-zA-Z0-9\-\.]*[a-zA-Z0-9])?$",
        );
    }

    #[test]
    fn generate_ipv4_format_matches_dotted_quad() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: ipv4"), &root, None),
            r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$",
        );
    }

    #[test]
    fn generate_ipv6_format_matches_ipv6_pattern() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: ipv6"), &root, None),
            r"^[0-9a-fA-F:]+$",
        );
    }

    #[test]
    fn generate_byte_format_matches_base64() {
        let root = yaml("{}");
        assert_matches(
            &generate(&yaml("type: string\nformat: byte"), &root, None),
            r"^[A-Za-z0-9+/]+=*$",
        );
    }

    #[test]
    fn generate_password_format_returns_non_empty_string() {
        let root = yaml("{}");
        let result = generate(&yaml("type: string\nformat: password"), &root, None);
        assert!(result.as_str().map(|s| !s.is_empty()).unwrap_or(false));
    }

    // --- $ref and allOf ---

    #[test]
    fn generate_follows_ref() {
        let root = yaml("Name:\n  type: string\n  example: Alice");
        let schema = yaml("$ref: '#/Name'");
        assert_eq!(generate(&schema, &root, None), json!("Alice"));
    }

    #[test]
    fn generate_merges_all_of() {
        let root = yaml("{}");
        let schema = yaml(
            "allOf:\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     a:\n\
             \x20       type: string\n\
             \x20       example: alpha\n\
             \x20 - type: object\n\
             \x20   properties:\n\
             \x20     b:\n\
             \x20       type: integer\n\
             \x20       example: 42",
        );
        let result = generate(&schema, &root, None);
        assert_eq!(result["a"], json!("alpha"));
        assert_eq!(result["b"], json!(42));
    }

    // --- forced variant ---

    #[test]
    fn generate_forced_variant_picks_correct_schema() {
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
        assert_eq!(generate(&schema, &root, Some("b"))["k"], json!("b"));
        assert_eq!(generate(&schema, &root, Some("a"))["k"], json!("a"));
    }
}

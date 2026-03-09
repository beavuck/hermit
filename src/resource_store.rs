use std::collections::HashMap;

use rand::RngExt;
use serde_json::Value;

pub struct CrudStore {
    items: HashMap<String, Value>,
    collections: HashMap<String, Vec<String>>,
}

impl Default for CrudStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CrudStore {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            collections: HashMap::new(),
        }
    }

    pub fn seed_item(&mut self, path: &str, value: Value) {
        if self.items.contains_key(path) {
            return;
        }
        let id = last_segment(path).to_string();
        let parent = parent_path(path).to_string();
        self.items.insert(path.to_string(), value);
        let ids = self.collections.entry(parent).or_default();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }

    pub fn get_item(&self, path: &str) -> Option<&Value> {
        self.items.get(path)
    }

    pub fn put_item(&mut self, path: &str, value: Value) {
        let id = last_segment(path).to_string();
        let parent = parent_path(path).to_string();
        self.items.insert(path.to_string(), value);
        let ids = self.collections.entry(parent).or_default();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }

    pub fn delete_item(&mut self, path: &str) {
        self.items.remove(path);
        let id = last_segment(path).to_string();
        let parent = parent_path(path).to_string();
        if let Some(ids) = self.collections.get_mut(&parent) {
            ids.retain(|i| i != &id);
        }
    }

    pub fn collection_items(&self, path: &str) -> Option<Vec<&Value>> {
        let ids = self.collections.get(path)?;
        Some(
            ids.iter()
                .filter_map(|id| self.items.get(&format!("{}/{}", path, id)))
                .collect(),
        )
    }

    pub fn collection_initialized(&self, path: &str) -> bool {
        self.collections.contains_key(path)
    }

    pub fn init_collection(&mut self, path: &str) {
        self.collections.entry(path.to_string()).or_default();
    }
}

pub fn new_uuid() -> String {
    let mut rng = rand::rng();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        rng.random::<u32>(),
        rng.random::<u16>(),
        rng.random::<u16>(),
        rng.random::<u16>(),
        rng.random::<u64>() & 0x0000_ffff_ffff_ffff,
    )
}

pub fn is_item_pattern(axum_path: &str) -> bool {
    axum_path
        .split('/')
        .next_back()
        .is_some_and(|s| s.starts_with('{'))
}

fn parent_path(path: &str) -> &str {
    match path.rfind('/') {
        Some(idx) if idx > 0 => &path[..idx],
        _ => "/",
    }
}

fn last_segment(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn seed_collection(store: &mut CrudStore, collection_path: &str, mock_body: &Option<Value>) {
    let items = extract_items_from_mock(mock_body);
    for item in items {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(new_uuid);
        let item_path = format!("{}/{}", collection_path, id);
        store.seed_item(&item_path, item);
    }
    store.init_collection(collection_path);
}

pub fn extract_items_from_mock(mock_body: &Option<Value>) -> Vec<Value> {
    match mock_body {
        Some(Value::Array(arr)) => arr.clone(),
        Some(Value::Object(obj)) => obj
            .values()
            .find(|v| v.is_array())
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn fill_to_count(template_items: Vec<Value>, min: usize, max: usize) -> Vec<Value> {
    if template_items.is_empty() {
        return Vec::new();
    }
    let target = rand::rng().random_range(min..=max);
    (0..target)
        .map(|i| {
            let mut item = template_items[i % template_items.len()].clone();
            if let Some(obj) = item.as_object_mut() {
                obj.insert("id".to_string(), Value::String(new_uuid()));
            }
            item
        })
        .collect()
}

pub fn build_collection_response(mock_body: &Option<Value>, items: Vec<&Value>) -> Value {
    let item_values: Vec<Value> = items.into_iter().cloned().collect();
    match mock_body {
        Some(Value::Array(_)) => Value::Array(item_values),
        Some(Value::Object(obj)) => {
            let mut result = obj.clone();
            if let Some(array_key) = obj
                .iter()
                .find(|(_, v)| v.is_array())
                .map(|(k, _)| k.clone())
            {
                result.insert(array_key, Value::Array(item_values));
            }
            Value::Object(result)
        }
        _ => Value::Array(item_values),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // --- is_item_pattern ---

    #[test]
    fn is_item_pattern_true_for_param_last_segment() {
        assert!(is_item_pattern("/projects/{projectId}"));
    }

    #[test]
    fn is_item_pattern_false_for_collection_path() {
        assert!(!is_item_pattern("/projects"));
    }

    #[test]
    fn is_item_pattern_false_for_nested_collection() {
        assert!(!is_item_pattern("/projects/{projectId}/tasks"));
    }

    #[test]
    fn is_item_pattern_true_for_nested_item() {
        assert!(is_item_pattern("/projects/{projectId}/tasks/{taskId}"));
    }

    // --- CrudStore ---

    #[test]
    fn put_item_stores_and_get_item_retrieves_it() {
        let mut store = CrudStore::new();
        store.put_item("/items/abc", json!({"id": "abc", "name": "test"}));
        assert_eq!(store.get_item("/items/abc").unwrap()["name"], json!("test"));
    }

    #[test]
    fn put_item_registers_in_parent_collection() {
        let mut store = CrudStore::new();
        store.put_item("/items/abc", json!({"id": "abc"}));
        let items = store.collection_items("/items").unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn delete_item_removes_from_store_and_collection() {
        let mut store = CrudStore::new();
        store.put_item("/items/abc", json!({"id": "abc"}));
        store.delete_item("/items/abc");
        assert!(store.get_item("/items/abc").is_none());
        assert_eq!(store.collection_items("/items").unwrap().len(), 0);
    }

    #[test]
    fn seed_item_does_not_overwrite_existing() {
        let mut store = CrudStore::new();
        store.put_item("/items/abc", json!({"id": "abc", "name": "original"}));
        store.seed_item("/items/abc", json!({"id": "abc", "name": "seeded"}));
        assert_eq!(
            store.get_item("/items/abc").unwrap()["name"],
            json!("original")
        );
    }

    #[test]
    fn collection_initialized_is_true_after_init() {
        let mut store = CrudStore::new();
        assert!(!store.collection_initialized("/items"));
        store.init_collection("/items");
        assert!(store.collection_initialized("/items"));
    }

    #[test]
    fn seed_collection_populates_items_and_collection() {
        let mut store = CrudStore::new();
        let mock = Some(json!([{"id": "x"}, {"id": "y"}]));
        seed_collection(&mut store, "/items", &mock);
        assert!(store.collection_initialized("/items"));
        assert_eq!(store.collection_items("/items").unwrap().len(), 2);
    }

    #[test]
    fn seed_collection_extracts_from_envelope_object() {
        let mut store = CrudStore::new();
        let mock = Some(json!({"total": 1, "items": [{"id": "z"}]}));
        seed_collection(&mut store, "/items", &mock);
        assert_eq!(store.collection_items("/items").unwrap().len(), 1);
    }

    // --- build_collection_response ---

    #[test]
    fn build_collection_response_replaces_array_field_in_envelope() {
        let mock = Some(json!({"total": 5, "page": 1, "items": [{"id": "old"}]}));
        let new_item = json!({"id": "new"});
        let result = build_collection_response(&mock, vec![&new_item]);
        assert_eq!(result["items"][0]["id"], json!("new"));
        assert_eq!(result["total"], json!(5));
    }

    #[test]
    fn build_collection_response_returns_array_for_bare_array_mock() {
        let mock = Some(json!([{"id": "old"}]));
        let new_item = json!({"id": "new"});
        let result = build_collection_response(&mock, vec![&new_item]);
        assert!(result.is_array());
        assert_eq!(result[0]["id"], json!("new"));
    }

    // --- new_uuid ---

    #[test]
    fn new_uuid_matches_uuid_pattern() {
        let uuid = new_uuid();
        let re =
            regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
                .unwrap();
        assert!(re.is_match(&uuid), "UUID {uuid:?} did not match pattern");
    }

    // --- fill_to_count ---

    #[test]
    fn fill_to_count_replicates_single_item_to_exact_target() {
        let items = vec![json!({"id": "a", "name": "first"})];
        let result = fill_to_count(items, 4, 4);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn fill_to_count_result_is_within_min_max_bounds() {
        for _ in 0..30 {
            let items = vec![json!({"id": "a"})];
            let result = fill_to_count(items.clone(), 2, 6);
            assert!(
                result.len() >= 2 && result.len() <= 6,
                "expected len in 2..=6, got {}",
                result.len()
            );
        }
    }

    #[test]
    fn fill_to_count_assigns_distinct_ids_to_replicated_items() {
        let items = vec![json!({"id": "a", "name": "x"})];
        let result = fill_to_count(items, 3, 3);
        let ids: Vec<_> = result.iter().filter_map(|v| v["id"].as_str()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 3, "expected 3 distinct ids, got {ids:?}");
    }
}

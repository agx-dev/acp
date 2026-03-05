use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::RwLock;

/// LRU cache for embeddings.
pub struct EmbeddingCache {
    cache: RwLock<LruCache>,
    max_entries: usize,
}

struct LruCache {
    entries: HashMap<String, CacheEntry>,
    order: Vec<String>,
}

struct CacheEntry {
    embedding: Vec<f32>,
}

impl EmbeddingCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache {
                entries: HashMap::new(),
                order: Vec::new(),
            }),
            max_entries,
        }
    }

    fn cache_key(model_id: &str, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model_id.as_bytes());
        hasher.update(b":");
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get(&self, model_id: &str, text: &str) -> Option<Vec<f32>> {
        let key = Self::cache_key(model_id, text);
        let cache = self.cache.read().ok()?;
        cache.entries.get(&key).map(|e| e.embedding.clone())
    }

    pub fn put(&self, model_id: &str, text: &str, embedding: Vec<f32>) {
        let key = Self::cache_key(model_id, text);

        if let Ok(mut cache) = self.cache.write() {
            if cache.entries.len() >= self.max_entries {
                if let Some(oldest_key) = cache.order.first().cloned() {
                    cache.entries.remove(&oldest_key);
                    cache.order.remove(0);
                }
            }

            cache
                .entries
                .insert(key.clone(), CacheEntry { embedding });
            cache.order.push(key);
        }
    }

    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.entries.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

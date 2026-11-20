//! Conditional-request cache.
//!
//! GitHub counts a `304 Not Modified` as *free* against the primary rate
//! limit, so re-validating a poll is the cheapest request available. The body
//! is kept here so a `304` can be answered without a second round trip.
//!
//! The cache is deliberately keyed by URL rather than by response type: the
//! only caller for a given URL always deserialises it the same way.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Enough for a few hundred repositories' worth of endpoints, which is far
/// beyond what a single poll cycle touches.
pub const DEFAULT_CAPACITY: usize = 512;

pub struct CachedBody {
    pub etag: String,
    pub body: Arc<Vec<u8>>,
    pub stored_at: Instant,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
    clock: u64,
}

struct Entry {
    etag: String,
    body: Arc<Vec<u8>>,
    stored_at: Instant,
    last_used: u64,
}

pub struct EtagCache {
    capacity: usize,
    inner: Mutex<Inner>,
}

impl EtagCache {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new(Inner::default()),
        }
    }

    #[must_use]
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// Returns the stored validator and body, marking the entry as recently
    /// used.
    pub fn get(&self, key: &str) -> Option<CachedBody> {
        let mut inner = self.inner.lock().ok()?;
        inner.clock += 1;
        let clock = inner.clock;

        let entry = inner.entries.get_mut(key)?;
        entry.last_used = clock;

        Some(CachedBody {
            etag: entry.etag.clone(),
            body: Arc::clone(&entry.body),
            stored_at: entry.stored_at,
        })
    }

    /// Only the validator, without disturbing the LRU order. Used to build an
    /// `If-None-Match` header before the request is sent.
    pub fn etag(&self, key: &str) -> Option<String> {
        let inner = self.inner.lock().ok()?;
        inner.entries.get(key).map(|entry| entry.etag.clone())
    }

    pub fn insert(&self, key: String, etag: String, body: Arc<Vec<u8>>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };

        inner.clock += 1;
        let clock = inner.clock;

        if inner.entries.len() >= self.capacity && !inner.entries.contains_key(&key) {
            if let Some(victim) = inner
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            {
                inner.entries.remove(&victim);
            }
        }

        inner.entries.insert(
            key,
            Entry {
                etag,
                body,
                stored_at: Instant::now(),
                last_used: clock,
            },
        );
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map_or(0, |inner| inner.entries.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: &str) -> Arc<Vec<u8>> {
        Arc::new(text.as_bytes().to_vec())
    }

    #[test]
    fn a_miss_is_a_miss() {
        let cache = EtagCache::new(4);
        assert!(cache.get("https://api.test/a").is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn stores_and_returns_the_validator_and_body() {
        let cache = EtagCache::new(4);
        cache.insert(
            "https://api.test/a".to_owned(),
            "\"v1\"".to_owned(),
            body("hello"),
        );

        let hit = cache.get("https://api.test/a").expect("hit");
        assert_eq!(hit.etag, "\"v1\"");
        assert_eq!(hit.body.as_slice(), b"hello");
    }

    #[test]
    fn evicts_the_least_recently_used_entry() {
        let cache = EtagCache::new(2);
        cache.insert("a".to_owned(), "1".to_owned(), body("a"));
        cache.insert("b".to_owned(), "2".to_owned(), body("b"));

        // Touch "a" so "b" becomes the coldest entry.
        assert!(cache.get("a").is_some());

        cache.insert("c".to_owned(), "3".to_owned(), body("c"));

        assert!(cache.get("a").is_some(), "the recently used entry survives");
        assert!(cache.get("b").is_none(), "the coldest entry was evicted");
        assert!(cache.get("c").is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn reinserting_a_key_does_not_evict() {
        let cache = EtagCache::new(1);
        cache.insert("a".to_owned(), "1".to_owned(), body("first"));
        cache.insert("a".to_owned(), "2".to_owned(), body("second"));

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("a").expect("hit").etag, "2");
    }

    #[test]
    fn the_validator_can_be_read_without_touching_the_lru_order() {
        let cache = EtagCache::new(2);
        cache.insert("a".to_owned(), "1".to_owned(), body("a"));
        cache.insert("b".to_owned(), "2".to_owned(), body("b"));

        assert_eq!(cache.etag("a").as_deref(), Some("1"));

        // "a" was only read for its validator, so "a" is still the coldest.
        cache.insert("c".to_owned(), "3".to_owned(), body("c"));
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
    }
}

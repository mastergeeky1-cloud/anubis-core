use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use xxhash_rust::xxh64::xxh64;

pub struct AudioCache {
    inner: Mutex<LruCache<u64, Vec<u8>>>,
}

impl AudioCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn make_key(text: &str, voice_id: &str) -> u64 {
        let mut data = Vec::with_capacity(text.len() + voice_id.len() + 1);
        data.extend_from_slice(text.as_bytes());
        data.push(0);
        data.extend_from_slice(voice_id.as_bytes());
        xxh64(&data, 0)
    }

    pub fn get(&self, key: u64) -> Option<Vec<u8>> {
        self.inner.lock().ok()?.get(&key).cloned()
    }

    pub fn insert(&self, key: u64, bytes: Vec<u8>) {
        if let Ok(mut c) = self.inner.lock() {
            c.put(key, bytes);
        }
    }
}

//! Per-user conversation memory for Noxis Core.
//!
//! Persists each user's turn history as JSON in SQLite (survives restarts),
//! with a small in-memory cache to avoid a DB read on every turn. Bounded per
//! user to `max_turns` (oldest dropped) to keep prompt size and storage sane.

use crate::db::Database;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// A single message in a conversation history.
pub type Turn = (String, String); // (role, content)

pub struct ConversationStore {
    db: Arc<Database>,
    /// In-memory cache keyed by user_id. The DB is the source of truth.
    cache: DashMap<i64, Vec<Turn>>,
    max_turns: usize,
}

impl ConversationStore {
    /// `max_turns` is the max user+assistant messages retained per user.
    pub fn new(db: Arc<Database>, max_turns: usize) -> Self {
        Self {
            db,
            cache: DashMap::new(),
            max_turns: max_turns.max(2),
        }
    }

    /// Append a user turn and an assistant turn, trimming to the window, and
    /// persist to SQLite.
    pub fn push(&self, user_id: i64, user: &str, assistant: &str) {
        let mut turns = self.history(user_id);
        turns.push(("user".to_string(), user.to_string()));
        turns.push(("assistant".to_string(), assistant.to_string()));
        let overflow = turns.len().saturating_sub(self.max_turns);
        if overflow > 0 {
            turns.drain(..overflow);
        }
        self.cache.insert(user_id, turns.clone());
        self.persist(user_id);
    }

    /// Snapshot of the current history (oldest first).
    pub fn history(&self, user_id: i64) -> Vec<Turn> {
        if let Some(c) = self.cache.get(&user_id) {
            return c.clone();
        } // Cache miss → load from DB once.
        let turns = self.load(user_id);
        self.cache.insert(user_id, turns.clone());
        turns
    }

    pub fn clear(&self, user_id: i64) {
        self.cache.remove(&user_id);
        self.persist(user_id);
    }

    fn persist(&self, user_id: i64) {
        let stored = self
            .cache
            .get(&user_id)
            .map(|r| r.value().clone())
            .unwrap_or_default();
        // Serialize as [[role, content], ...]
        let arr: Vec<Vec<&str>> = stored
            .iter()
            .map(|(r, c)| vec![r.as_str(), c.as_str()])
            .collect();
        if let Ok(json) = serde_json::to_string(&arr) {
            let _ = self.db.save_memory(user_id, &json);
        }
    }

    fn load(&self, user_id: i64) -> Vec<Turn> {
        let raw = self.db.load_memory(user_id).unwrap_or_else(|_| "[]".into());
        let parsed: Vec<Turn> = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|item| {
                        let pair = item.as_array()?;
                        let role = pair.first()?.as_str()?.to_string();
                        let content = pair.get(1)?.as_str()?.to_string();
                        Some((role, content))
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Trim to window defensively.
        let overflow = parsed.len().saturating_sub(self.max_turns);
        let mut out = parsed;
        if overflow > 0 {
            out.drain(..overflow);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db() -> Arc<Database> {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = format!("/tmp/anubis_mem_test_{n}.db");
        let _ = std::fs::remove_file(&path);
        Arc::new(Database::open(&path, 4).expect("open"))
    }

    #[test]
    fn separate_users_do_not_interfere() {
        let store = ConversationStore::new(test_db(), 12);
        store.push(1, "hi", "hello");
        store.push(2, "yo", "hey");
        assert_eq!(store.history(1).len(), 2);
        assert_eq!(store.history(2).len(), 2);
        assert_eq!(store.history(1).last().unwrap().1, "hello");
        assert_eq!(store.history(2).last().unwrap().1, "hey");
    }

    #[test]
    fn bounds_history_to_window() {
        let store = ConversationStore::new(test_db(), 4);
        for _ in 0..5 {
            store.push(7, "u", "a");
        }
        // 5 pushes * 2 = 10 turns, trimmed to window 4.
        assert!(store.history(7).len() <= 4);
    }

    #[test]
    fn persists_across_store_instances() {
        let db = test_db();
        {
            let store = ConversationStore::new(db.clone(), 12);
            store.push(9, "persist", "me");
        }
        let store2 = ConversationStore::new(db.clone(), 12);
        // Fresh store with no cache → reads from DB.
        assert_eq!(
            store2.history(9),
            vec![
                ("user".into(), "persist".into()),
                ("assistant".into(), "me".into())
            ]
        );
    }

    #[test]
    fn clear_empties_persisted_memory() {
        let db = test_db();
        {
            let store = ConversationStore::new(db.clone(), 12);
            store.push(5, "a", "b");
            store.clear(5);
        }
        let store2 = ConversationStore::new(db.clone(), 12);
        assert!(store2.history(5).is_empty());
    }
}

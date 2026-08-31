//! Per-user short-term conversation memory for Noxis Core.
//!
//! Keeps the last `N` turns (role + content) per user so the brain holds
//! context across `/ask` calls and voice-to-voice turns. Ring-buffer style,
//! bounded per user to avoid unbounded growth.

use dashmap::DashMap;

/// A single message in a conversation history.
pub type Turn = (String, String); // (role, content)

pub struct ConversationStore {
    inner: DashMap<i64, Vec<Turn>>,
    max_turns: usize,
}

impl ConversationStore {
    /// `max_turns` is the max number of user+assistant messages retained per
    /// user (oldest first).
    pub fn new(max_turns: usize) -> Self {
        Self {
            inner: DashMap::new(),
            max_turns: max_turns.max(2),
        }
    }

    /// Append a user turn and an assistant turn, trimming to the window.
    pub fn push(&self, user_id: i64, user: &str, assistant: &str) {
        let mut entry = self.inner.entry(user_id).or_default();
        let v = entry.value_mut();
        v.push(("user".to_string(), user.to_string()));
        v.push(("assistant".to_string(), assistant.to_string()));
        let overflow = v.len().saturating_sub(self.max_turns);
        if overflow > 0 {
            v.drain(..overflow);
        }
    }

    /// Snapshot of the current history (oldest first) without a leading user turn.
    pub fn history(&self, user_id: i64) -> Vec<Turn> {
        self.inner
            .get(&user_id)
            .map(|e| e.value().clone())
            .unwrap_or_default()
    }

    pub fn clear(&self, user_id: i64) {
        self.inner.remove(&user_id);
    }
}

impl Default for ConversationStore {
    fn default() -> Self {
        Self::new(12)
    }
}

#[cfg(test)]
mod tests {
    use super::ConversationStore;

    #[test]
    fn bounds_history_to_window() {
        let store = ConversationStore::new(4);
        store.push(1, "a", "A");
        store.push(1, "b", "B");
        store.push(1, "c", "C");
        let h = store.history(1);
        // 3 user + 3 assistant = 6 turns; keep last 4.
        assert_eq!(h.len(), 4);
        assert_eq!(h[0], ("user".to_string(), "b".to_string()));
        assert_eq!(
            h.last().unwrap(),
            &("assistant".to_string(), "C".to_string())
        );
    }

    #[test]
    fn separate_users_do_not_interfere() {
        let store = ConversationStore::new(4);
        store.push(1, "hi", "hello");
        store.push(2, "bonjour", "salut");
        assert_eq!(store.history(1).len(), 2);
        assert_eq!(store.history(2).len(), 2);
    }
}

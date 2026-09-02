use crate::error::{AnubisError, Result};
use crate::tts::voices;
use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

type Conn = r2d2::PooledConnection<SqliteConnectionManager>;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct UserRow {
    pub id: i64,
    pub username: Option<String>,
    pub lang: String,
    pub credits: i32,
    pub daily_used: i32,
    pub daily_reset: String,
    pub active_voice: String,
    pub teacher_mode: bool,
}

const MIGRATION: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id           INTEGER PRIMARY KEY,
    username     TEXT,
    lang         TEXT    NOT NULL DEFAULT 'en',
    credits      INTEGER NOT NULL DEFAULT 3,
    daily_used   INTEGER NOT NULL DEFAULT 0,
    daily_reset  TEXT    NOT NULL DEFAULT '',
    active_voice TEXT    NOT NULL DEFAULT '',
    memory       TEXT    NOT NULL DEFAULT '[]',
    teacher_mode INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS credit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    delta      INTEGER NOT NULL,
    reason     TEXT    NOT NULL,
    created_at TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS payments (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    charge_id          TEXT    NOT NULL UNIQUE,
    payload            TEXT    NOT NULL,
    user_id            INTEGER NOT NULL REFERENCES users(id),
    credits            INTEGER NOT NULL,
    stars              INTEGER NOT NULL,
    created_at         TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_credit_log_user ON credit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_payments_user   ON payments(user_id);
";

impl Database {
    pub fn open(path: &str, max_size: u32) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path).with_init(|c| {
            c.execute_batch(MIGRATION)?;
            // Idempotent migrations for databases created before a column was
            // added to `users` (CREATE TABLE IF NOT EXISTS won't alter existing
            // tables).
            let has_teacher: i64 = c.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name='teacher_mode'",
                [],
                |r| r.get(0),
            )?;
            if has_teacher == 0 {
                c.execute(
                    "ALTER TABLE users ADD COLUMN teacher_mode INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
            }
            Ok(())
        });
        let pool = Pool::builder()
            .max_size(max_size)
            .build(manager)
            .map_err(AnubisError::from)?;
        Ok(Self { pool })
    }

    pub fn conn(&self) -> Result<Conn> {
        self.pool.get().map_err(AnubisError::from)
    }

    pub fn upsert_user(&self, id: i64, username: Option<&str>) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO users (id, username) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET username = excluded.username",
            params![id, username],
        )?;
        Ok(())
    }

    pub fn set_lang(&self, user_id: i64, lang: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET lang = ?1 WHERE id = ?2",
            params![lang, user_id],
        )?;
        Ok(())
    }

    /// Get user's active voice.
    pub fn active_voice(&self, user_id: i64) -> String {
        let Ok(c) = self.conn() else {
            return voices::default_for_lang("en").to_string();
        };
        c.query_row(
            "SELECT active_voice FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| voices::default_for_lang("en").to_string())
    }

    /// Get user's current language.
    pub fn user_lang(&self, user_id: i64) -> String {
        let Ok(c) = self.conn() else {
            return "en".to_string();
        };
        c.query_row(
            "SELECT lang FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "en".to_string())
    }

    pub fn set_active_voice(&self, user_id: i64, voice_id: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET active_voice = ?1 WHERE id = ?2",
            params![voice_id, user_id],
        )?;
        Ok(())
    }

    pub fn set_teacher_mode(&self, user_id: i64, enabled: bool) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET teacher_mode = ?1 WHERE id = ?2",
            params![enabled as i32, user_id],
        )?;
        Ok(())
    }

    pub fn teacher_mode(&self, user_id: i64) -> bool {
        let Ok(c) = self.conn() else {
            return false;
        };
        c.query_row(
            "SELECT teacher_mode FROM users WHERE id = ?1",
            params![user_id],
            |r| {
                let v: i32 = r.get(0)?;
                Ok(v != 0)
            },
        )
        .unwrap_or(false)
    }

    /// Persist the full conversation memory JSON for a user.
    pub fn save_memory(&self, user_id: i64, memory_json: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO users (id, memory) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET memory = excluded.memory",
            params![user_id, memory_json],
        )?;
        Ok(())
    }

    /// Load the conversation memory JSON for a user (returns "[]" if missing).
    pub fn load_memory(&self, user_id: i64) -> Result<String> {
        let c = self.conn()?;
        let res = c.query_row(
            "SELECT COALESCE(memory, '[]') FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, String>(0),
        );
        match res {
            Ok(s) => Ok(s),
            Err(_) => Ok("[]".to_string()),
        }
    }

    /// Fetch the full user row.
    pub fn get_user(&self, user_id: i64) -> Result<Option<UserRow>> {
        let c = self.conn()?;
        let mut stmt = c.prepare(
            "SELECT id, username, lang, credits, daily_used, daily_reset,
                    active_voice, teacher_mode
             FROM users WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![user_id], |row| {
            Ok(UserRow {
                id: row.get(0)?,
                username: row.get(1)?,
                lang: row.get(2)?,
                credits: row.get(3)?,
                daily_used: row.get(4)?,
                daily_reset: row.get(5)?,
                active_voice: row.get(6)?,
                teacher_mode: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    // ── Credits / Payments ────────────────────────────────────────────────

    /// Spend a daily free credit. Returns true if a credit was consumed,
    /// false if the daily quota is exhausted (and not in unlimited mode).
    pub fn consume_credit(&self, user_id: i64, free_daily: i32, unlimited: bool) -> Result<bool> {
        let c = self.conn()?;
        let today = Utc::now().format("%Y-%m-%d").to_string();

        // Reset daily counter if the day rolled over.
        c.execute(
            "UPDATE users SET daily_used = 0, daily_reset = ?1
             WHERE id = ?2 AND daily_reset != ?1",
            params![today, user_id],
        )?;

        if unlimited {
            return Ok(true);
        }

        let used: i32 = c
            .query_row(
                "SELECT daily_used FROM users WHERE id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if used >= free_daily {
            return Ok(false);
        }

        c.execute(
            "UPDATE users SET daily_used = daily_used + 1 WHERE id = ?1",
            params![user_id],
        )?;
        Ok(true)
    }

    /// Grant bonus credits (from a Stars purchase or admin grant).
    pub fn add_credits(&self, user_id: i64, amount: i32, reason: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET credits = credits + ?1 WHERE id = ?2",
            params![amount, user_id],
        )?;
        c.execute(
            "INSERT INTO credit_log (user_id, delta, reason, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![user_id, amount, reason, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Record a Telegram Stars payment (idempotent on charge_id).
    pub fn record_payment(
        &self,
        user_id: i64,
        charge_id: &str,
        payload: &str,
        credits: i32,
        stars: i32,
    ) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT OR IGNORE INTO payments (charge_id, payload, user_id, credits, stars, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                charge_id,
                payload,
                user_id,
                credits,
                stars,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn test_db() -> Database {
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt::try_init();
        });
        Database::open(":memory:", 4).unwrap()
    }

    #[test]
    fn upsert_user_and_get_lang() {
        let db = test_db();
        db.upsert_user(123, Some("alice")).unwrap();
        assert_eq!(db.user_lang(123), "en");
        db.set_lang(123, "ar").unwrap();
        assert_eq!(db.user_lang(123), "ar");
    }

    #[test]
    fn active_voice_defaults_for_lang() {
        let db = test_db();
        let voice = db.active_voice(999);
        assert!(
            !voice.is_empty(),
            "should return a default voice for unknown user"
        );
    }

    #[test]
    fn set_active_voice() {
        let db = test_db();
        db.upsert_user(10, None).unwrap();
        db.set_active_voice(10, "it_federico-medium").unwrap();
        assert_eq!(db.active_voice(10), "it_federico-medium");
    }

    #[test]
    fn teacher_mode_toggle() {
        let db = test_db();
        db.upsert_user(20, None).unwrap();
        assert!(!db.teacher_mode(20));
        db.set_teacher_mode(20, true).unwrap();
        assert!(db.teacher_mode(20));
        db.set_teacher_mode(20, false).unwrap();
        assert!(!db.teacher_mode(20));
    }

    #[test]
    fn memory_round_trip() {
        let db = test_db();
        db.upsert_user(30, None).unwrap();
        db.save_memory(30, r#"[{"role":"user","content":"hi"}]"#)
            .unwrap();
        let json = db.load_memory(30).unwrap();
        assert!(json.contains("hi"));
    }

    #[test]
    fn memory_missing_user_returns_empty_array() {
        let db = test_db();
        assert_eq!(db.load_memory(999).unwrap(), "[]");
    }
}

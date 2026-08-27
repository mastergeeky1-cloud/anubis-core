use crate::error::{AnubisError, Result};
use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

type Conn = r2d2::PooledConnection<SqliteConnectionManager>;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

// Pool is internally Arc-based and is Send + Sync.
unsafe impl Send for Database {}
unsafe impl Sync for Database {}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub lang: String,
    pub credits: i32,
    pub daily_used: i32,
    pub daily_reset: String,
    pub active_voice: String,
    pub consent_at: Option<String>,
    pub banned: bool,
}

#[derive(Debug, Clone)]
pub struct VoiceClone {
    pub id: String,
    pub user_id: i64,
    pub name: String,
    pub wav_path: String,
    pub ref_text: String,
    pub created_at: String,
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
    active_voice TEXT    NOT NULL DEFAULT 'en_US-amy-medium',
    consent_at   TEXT,
    banned       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS voice_clones (
    id         TEXT    PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    name       TEXT    NOT NULL,
    wav_path   TEXT    NOT NULL,
    ref_text   TEXT    NOT NULL DEFAULT '',
    created_at TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS credit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    delta      INTEGER NOT NULL,
    reason     TEXT    NOT NULL,
    created_at TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ts         TEXT    NOT NULL,
    user_id    INTEGER NOT NULL,
    action     TEXT    NOT NULL,
    detail     TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_clones_user     ON voice_clones(user_id);
CREATE INDEX IF NOT EXISTS idx_credit_log_user ON credit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_user      ON audit_log(user_id);
";

impl Database {
    pub fn open(path: &str, max_size: u32) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path).with_init(|c| {
            c.execute_batch(MIGRATION)?;
            Ok(())
        });
        let pool = Pool::builder()
            .max_size(max_size)
            .build(manager)
            .map_err(AnubisError::from)?;
        Ok(Self { pool })
    }

    fn conn(&self) -> Result<Conn> {
        self.pool.get().map_err(AnubisError::from)
    }

    /// Append-only security/audit trail.
    pub fn audit(&self, user_id: i64, action: &str, detail: &str) {
        if let Ok(c) = self.conn() {
            let _ = c.execute(
                "INSERT INTO audit_log (ts, user_id, action, detail) VALUES (?1, ?2, ?3, ?4)",
                params![Utc::now().to_rfc3339(), user_id, action, detail],
            );
        }
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

    pub fn get_user(&self, id: i64) -> Result<Option<User>> {
        let c = self.conn()?;
        let mut stmt = c.prepare(
            "SELECT id, username, lang, credits, daily_used, daily_reset,
                    active_voice, consent_at, banned
             FROM users WHERE id = ?1",
        )?;
        let res = stmt.query_row(params![id], |r| {
            Ok(User {
                id: r.get(0)?,
                username: r.get(1)?,
                lang: r.get(2)?,
                credits: r.get(3)?,
                daily_used: r.get(4)?,
                daily_reset: r.get(5)?,
                active_voice: r.get(6)?,
                consent_at: r.get(7)?,
                banned: r.get::<_, i32>(8)? != 0,
            })
        });
        match res {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_lang(&self, user_id: i64, lang: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET lang = ?1 WHERE id = ?2",
            params![lang, user_id],
        )?;
        Ok(())
    }

    pub fn set_active_voice(&self, user_id: i64, voice_id: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET active_voice = ?1 WHERE id = ?2",
            params![voice_id, user_id],
        )?;
        Ok(())
    }

    pub fn has_consent(&self, user_id: i64) -> bool {
        let Ok(c) = self.conn() else { return false };
        c.query_row(
            "SELECT 1 FROM users WHERE id = ?1 AND consent_at IS NOT NULL",
            params![user_id],
            |_| Ok(()),
        )
        .is_ok()
    }

    pub fn set_consent(&self, user_id: i64) -> Result<()> {
        let c = self.conn()?;
        let now = Utc::now().to_rfc3339();
        c.execute(
            "UPDATE users SET consent_at = ?1 WHERE id = ?2",
            params![now, user_id],
        )?;
        Ok(())
    }

    pub fn is_banned(&self, user_id: i64) -> bool {
        let Ok(c) = self.conn() else { return false };
        c.query_row(
            "SELECT banned FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false)
    }

    pub fn ban_user(&self, user_id: i64, banned: bool) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET banned = ?1 WHERE id = ?2",
            params![banned as i32, user_id],
        )?;
        Ok(())
    }

    pub fn consume_credit(&self, user_id: i64, free_daily: i32, unlimited: bool) -> Result<bool> {
        if unlimited {
            return Ok(true);
        }
        let user = match self.get_user(user_id)? {
            Some(u) => u,
            None => return Ok(false),
        };
        let today = Utc::now().date_naive().to_string();
        let c = self.conn()?;
        let daily_used = if user.daily_reset != today {
            c.execute(
                "UPDATE users SET daily_used = 0, daily_reset = ?1 WHERE id = ?2",
                params![today, user_id],
            )?;
            0
        } else {
            user.daily_used
        };
        if daily_used < free_daily {
            c.execute(
                "UPDATE users SET daily_used = daily_used + 1 WHERE id = ?1",
                params![user_id],
            )?;
            self.log_credit_conn(&c, user_id, -1, "daily_free")?;
            return Ok(true);
        }
        if user.credits > 0 {
            c.execute(
                "UPDATE users SET credits = credits - 1 WHERE id = ?1",
                params![user_id],
            )?;
            self.log_credit_conn(&c, user_id, -1, "paid_credit")?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn add_credits(&self, user_id: i64, amount: i32, reason: &str) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE users SET credits = credits + ?1 WHERE id = ?2",
            params![amount, user_id],
        )?;
        self.log_credit_conn(&c, user_id, amount, reason)
    }

    fn log_credit_conn(
        &self,
        c: &rusqlite::Connection,
        user_id: i64,
        delta: i32,
        reason: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        c.execute(
            "INSERT INTO credit_log (user_id, delta, reason, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, delta, reason, now],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> Result<(i64, i64)> {
        let c = self.conn()?;
        let users: i64 = c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        let gens: i64 = c.query_row(
            "SELECT COALESCE(SUM(ABS(delta)), 0) FROM credit_log WHERE delta < 0",
            [],
            |r| r.get(0),
        )?;
        Ok((users, gens))
    }

    pub fn save_clone(&self, clone: &VoiceClone) -> Result<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT OR REPLACE INTO voice_clones (id, user_id, name, wav_path, ref_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![clone.id, clone.user_id, clone.name, clone.wav_path, clone.ref_text, clone.created_at],
        )?;
        Ok(())
    }

    pub fn get_clones(&self, user_id: i64) -> Result<Vec<VoiceClone>> {
        let c = self.conn()?;
        let mut stmt = c.prepare(
            "SELECT id, user_id, name, wav_path, ref_text, created_at
             FROM voice_clones WHERE user_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |r| {
                Ok(VoiceClone {
                    id: r.get(0)?,
                    user_id: r.get(1)?,
                    name: r.get(2)?,
                    wav_path: r.get(3)?,
                    ref_text: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_latest_clone(&self, user_id: i64) -> Result<Option<VoiceClone>> {
        let c = self.conn()?;
        let mut stmt = c.prepare(
            "SELECT id, user_id, name, wav_path, ref_text, created_at
             FROM voice_clones WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let res = stmt.query_row(params![user_id], |r| {
            Ok(VoiceClone {
                id: r.get(0)?,
                user_id: r.get(1)?,
                name: r.get(2)?,
                wav_path: r.get(3)?,
                ref_text: r.get(4)?,
                created_at: r.get(5)?,
            })
        });
        match res {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_clone(&self, clone_id: &str, user_id: i64) -> Result<bool> {
        let c = self.conn()?;
        let n = c.execute(
            "DELETE FROM voice_clones WHERE id = ?1 AND user_id = ?2",
            params![clone_id, user_id],
        )?;
        Ok(n > 0)
    }
}

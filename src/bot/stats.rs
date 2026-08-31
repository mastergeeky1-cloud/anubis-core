//! Statistics handlers for user and admin commands

use crate::db::Database;
use crate::i18n::Strings;
use chrono::{Duration, Utc};
use std::sync::Arc;

/// Stats returned to the user (/mystats)
#[derive(Debug, Clone)]
pub struct UserStats {
    pub total_generations: i64,
    pub total_clones: i64,
    pub credits: i32,
    pub daily_used: i32,
    pub free_daily_limit: i32,
    pub member_since: Option<String>,
    pub last_active: Option<String>,
}

/// Detailed stats for admin (/stats)
#[derive(Debug, Clone)]
pub struct AdminStats {
    pub total_users: i64,
    pub total_generations: i64,
    pub total_clones: i64,
    pub active_today: i64,
    pub active_this_week: i64,
    pub active_this_month: i64,
    pub users_with_consent: i64,
    pub banned_users: i64,
    pub top_users: Vec<TopUser>,
}

/// Top user entry for admin stats
#[derive(Debug, Clone)]
pub struct TopUser {
    pub user_id: i64,
    pub username: Option<String>,
    pub generations: i64,
    pub clones: i64,
}

/// User list entry for /users command
#[derive(Debug, Clone)]
pub struct UserEntry {
    pub user_id: i64,
    pub username: Option<String>,
    pub credits: i32,
    pub daily_used: i32,
    pub lang: String,
    pub consent: bool,
    pub banned: bool,
    pub last_active: Option<String>,
}

pub struct StatsEngine {
    db: Arc<Database>,
}

impl StatsEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get stats for a specific user (/mystats)
    pub async fn user_stats(
        &self,
        user_id: i64,
        free_daily_limit: i32,
    ) -> anyhow::Result<UserStats> {
        let conn = self.db.conn()?;

        // Total generations (credit_log with negative delta)
        let total_generations: i64 = conn.query_row(
            "SELECT COALESCE(SUM(ABS(delta)), 0) FROM credit_log WHERE user_id = ?1 AND delta < 0",
            [user_id],
            |r| r.get(0),
        )?;

        // Total clones
        let total_clones: i64 = conn.query_row(
            "SELECT COUNT(*) FROM voice_clones WHERE user_id = ?1",
            [user_id],
            |r| r.get(0),
        )?;

        // User details
        let (credits, daily_used, _lang, _consent_at, _banned, member_since, last_active) = conn
            .query_row(
                "SELECT credits, daily_used, lang, consent_at, banned, daily_reset, 
                   (SELECT MAX(ts) FROM audit_log WHERE user_id = users.id) as last_active
             FROM users WHERE id = ?1",
                [user_id],
                |r| {
                    Ok((
                        r.get::<_, i32>(0)?,
                        r.get::<_, i32>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, i32>(4)? != 0,
                        r.get::<_, String>(5)?,
                        r.get::<_, Option<String>>(6)?,
                    ))
                },
            )?;

        Ok(UserStats {
            total_generations,
            total_clones,
            credits,
            daily_used,
            free_daily_limit,
            member_since: if member_since.is_empty() {
                None
            } else {
                Some(member_since)
            },
            last_active,
        })
    }

    /// Get comprehensive admin stats (/stats)
    pub async fn admin_stats(&self) -> anyhow::Result<AdminStats> {
        let conn = self.db.conn()?;

        let total_users: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        let total_generations: i64 = conn.query_row(
            "SELECT COALESCE(SUM(ABS(delta)), 0) FROM credit_log WHERE delta < 0",
            [],
            |r| r.get(0),
        )?;
        let total_clones: i64 =
            conn.query_row("SELECT COUNT(*) FROM voice_clones", [], |r| r.get(0))?;

        // Active today
        let today = Utc::now().date_naive().to_string();
        let active_today: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT user_id) FROM audit_log WHERE date(ts) = ?1",
            [today.clone()],
            |r| r.get(0),
        )?;

        // Active this week
        let week_ago = (Utc::now() - Duration::days(7)).date_naive().to_string();
        let active_this_week: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT user_id) FROM audit_log WHERE date(ts) >= ?1",
            [week_ago],
            |r| r.get(0),
        )?;

        // Active this month
        let month_ago = (Utc::now() - Duration::days(30)).date_naive().to_string();
        let active_this_month: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT user_id) FROM audit_log WHERE date(ts) >= ?1",
            [month_ago],
            |r| r.get(0),
        )?;

        let users_with_consent: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE consent_at IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let banned_users: i64 =
            conn.query_row("SELECT COUNT(*) FROM users WHERE banned = 1", [], |r| {
                r.get(0)
            })?;

        // Top 10 users by generations
        let mut stmt = conn.prepare(
            "SELECT u.id, u.username, 
                    COALESCE(SUM(CASE WHEN cl.delta < 0 THEN ABS(cl.delta) ELSE 0 END), 0) as gens,
                    COUNT(vc.id) as clones
             FROM users u
             LEFT JOIN credit_log cl ON cl.user_id = u.id
             LEFT JOIN voice_clones vc ON vc.user_id = u.id
             GROUP BY u.id, u.username
             ORDER BY gens DESC
             LIMIT 10",
        )?;
        let top_users = stmt
            .query_map([], |r| {
                Ok(TopUser {
                    user_id: r.get(0)?,
                    username: r.get(1)?,
                    generations: r.get(2)?,
                    clones: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AdminStats {
            total_users,
            total_generations,
            total_clones,
            active_today,
            active_this_week,
            active_this_month,
            users_with_consent,
            banned_users,
            top_users,
        })
    }

    /// Get all users for /users command
    pub async fn all_users(&self) -> anyhow::Result<Vec<UserEntry>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT u.id, u.username, u.credits, u.daily_used, u.lang, 
                    u.consent_at IS NOT NULL as has_consent, u.banned,
                    (SELECT MAX(ts) FROM audit_log WHERE user_id = u.id) as last_active
             FROM users u
             ORDER BY u.id",
        )?;
        let users = stmt
            .query_map([], |r| {
                Ok(UserEntry {
                    user_id: r.get(0)?,
                    username: r.get(1)?,
                    credits: r.get(2)?,
                    daily_used: r.get(3)?,
                    lang: r.get(4)?,
                    consent: r.get(5)?,
                    banned: r.get::<_, i32>(6)? != 0,
                    last_active: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    /// Get daily active users for /dailyactive command
    pub async fn daily_active(&self, days: i64) -> anyhow::Result<Vec<(String, i64)>> {
        let conn = self.db.conn()?;
        let mut results = Vec::new();

        for i in 0..days {
            let date = (Utc::now() - Duration::days(i)).date_naive().to_string();
            let count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT user_id) FROM audit_log WHERE date(ts) = ?1",
                [date.clone()],
                |r| r.get(0),
            )?;
            results.push((date, count));
        }

        Ok(results)
    }
}

/// Format user stats for display
pub fn format_user_stats(stats: &UserStats, s: &Strings) -> String {
    let since = stats.member_since.as_deref().unwrap_or("N/A");
    let last = stats.last_active.as_deref().unwrap_or("N/A");
    format!(
        "{}\n\n{} {} / {}\n{} {}\n{} {} / {}\n{} {}\n{} {}",
        s.mystats_header,
        s.mystats_generations,
        stats.total_generations,
        stats.total_clones,
        s.mystats_credits,
        stats.credits,
        s.mystats_daily,
        stats.daily_used,
        stats.free_daily_limit,
        s.mystats_since,
        since,
        s.mystats_last,
        last,
    )
}

/// Format admin stats for display
pub fn format_admin_stats(stats: &AdminStats, s: &Strings) -> String {
    let mut top = String::new();
    for (i, u) in stats.top_users.iter().enumerate() {
        let name = u.username.as_deref().unwrap_or("—");
        top.push_str(&format!(
            "{}. {} (ID: {}) — {} gens, {} clones\n",
            i + 1,
            name,
            u.user_id,
            u.generations,
            u.clones
        ));
    }
    if top.is_empty() {
        top = "—".to_string();
    }

    format!(
        "{}\n\n👥 Total users: {}\n🎤 Total generations: {}\n🎭 Total clones: {}\n📅 Active today: {}\n📅 Active this week: {}\n📅 Active this month: {}\n✅ Users with consent: {}\n🚫 Banned users: {}\n\n🏆 Top users:\n{}",
        s.stats_header,
        stats.total_users,
        stats.total_generations,
        stats.total_clones,
        stats.active_today,
        stats.active_this_week,
        stats.active_this_month,
        stats.users_with_consent,
        stats.banned_users,
        top
    )
}

/// Format user list for display
pub fn format_user_list(users: &[UserEntry], s: &Strings) -> String {
    let mut out = format!("{} ({} total)\n\n", s.users_header, users.len());
    for u in users {
        let status = if u.banned {
            "🚫"
        } else if u.consent {
            "✅"
        } else {
            "⏳"
        };
        let name = u.username.as_deref().unwrap_or("—");
        out.push_str(&format!(
            "{} ID:{} @{} | {} cred | {} daily | {} | {}\n",
            status,
            u.user_id,
            name,
            u.credits,
            u.daily_used,
            u.lang,
            u.last_active.as_deref().unwrap_or("never")
        ));
    }
    out
}

/// Format daily active for display
pub fn format_daily_active(data: &[(String, i64)], s: &Strings) -> String {
    let mut out = format!("{}\n\n", s.daily_active_header);
    for (date, count) in data {
        out.push_str(&format!("📅 {} — {} users\n", date, count));
    }
    out
}

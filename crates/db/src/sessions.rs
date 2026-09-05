//! Repository نشست‌های کاربر — مدیریت دستگاه‌های واردشده (فاز ارتقا U8).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Session {
    pub token: String,
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

impl Session {
    /// نام دوستانه‌ی دستگاه/مرورگر از روی User-Agent.
    pub fn device_label(&self) -> String {
        let ua = self.user_agent.as_deref().unwrap_or("");
        let os = if ua.contains("Android") {
            "اندروید"
        } else if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iOS") {
            "آی‌اواس"
        } else if ua.contains("Windows") {
            "ویندوز"
        } else if ua.contains("Mac OS") || ua.contains("Macintosh") {
            "مک"
        } else if ua.contains("Linux") {
            "لینوکس"
        } else {
            ""
        };
        let browser = if ua.contains("Edg") {
            "Edge"
        } else if ua.contains("Chrome") || ua.contains("CriOS") {
            "Chrome"
        } else if ua.contains("Firefox") || ua.contains("FxiOS") {
            "Firefox"
        } else if ua.contains("Safari") {
            "Safari"
        } else {
            ""
        };
        match (browser.is_empty(), os.is_empty()) {
            (false, false) => format!("{browser} روی {os}"),
            (false, true) => browser.to_string(),
            (true, false) => os.to_string(),
            (true, true) => "دستگاه نامشخص".to_string(),
        }
    }

    /// آیکن مناسب نوع دستگاه.
    pub fn device_icon(&self) -> &'static str {
        let ua = self.user_agent.as_deref().unwrap_or("");
        if ua.contains("Android") || ua.contains("iPhone") || ua.contains("Mobile") {
            "📱"
        } else if ua.contains("iPad") || ua.contains("Tablet") {
            "📲"
        } else {
            "💻"
        }
    }

    pub fn ip_fa(&self) -> String {
        match &self.ip {
            Some(ip) if !ip.is_empty() => domain::fa_digits(ip),
            _ => "نامشخص".to_string(),
        }
    }

    /// «آخرین فعالیت» به‌صورت نسبی فارسی.
    pub fn last_seen_relative_fa(&self) -> String {
        rel_fa(self.last_seen_at)
    }

    /// «تاریخ ورود» به‌صورت نسبی فارسی.
    pub fn created_relative_fa(&self) -> String {
        rel_fa(self.created_at)
    }
}

fn rel_fa(t: DateTime<Utc>) -> String {
    let d = Utc::now().signed_duration_since(t);
    let mins = d.num_minutes();
    let hours = d.num_hours();
    let days = d.num_days();
    let s = if mins < 1 {
        "هم‌اکنون".to_string()
    } else if mins < 60 {
        format!("{mins} دقیقه پیش")
    } else if hours < 24 {
        format!("{hours} ساعت پیش")
    } else if days == 1 {
        "دیروز".to_string()
    } else if days < 7 {
        format!("{days} روز پیش")
    } else if days < 30 {
        format!("{} هفته پیش", days / 7)
    } else if days < 365 {
        format!("{} ماه پیش", days / 30)
    } else {
        format!("{} سال پیش", days / 365)
    };
    domain::fa_digits(&s)
}

/// ساخت نشست جدید (هنگام ورود موفق). توکن باید از پیش تولید شده باشد.
pub async fn create(
    db: &Db,
    token: &str,
    user_id: Uuid,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_sessions (token, user_id, user_agent, ip) VALUES ($1,$2,$3,$4)",
    )
    .bind(token)
    .bind(user_id)
    .bind(user_agent)
    .bind(ip)
    .execute(db)
    .await?;
    Ok(())
}

/// یافتن user_id یک نشست معتبر و به‌روزرسانیِ کم‌هزینه‌ی last_seen
/// (فقط اگر بیش از ۵ دقیقه از آخرین به‌روزرسانی گذشته باشد — برای کاهش نوشتن).
pub async fn user_for_token(db: &Db, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid,)>(
        "UPDATE user_sessions SET last_seen_at = now()
         WHERE token = $1 AND last_seen_at < now() - interval '5 minutes'
         RETURNING user_id",
    )
    .bind(token)
    .fetch_optional(db)
    .await?;
    if let Some((uid,)) = row {
        return Ok(Some(uid));
    }
    // یا نشست تازه به‌روز شده، یا وجود ندارد.
    sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM user_sessions WHERE token = $1")
        .bind(token)
        .fetch_optional(db)
        .await
}

/// همه‌ی نشست‌های فعال یک کاربر — جدیدترین فعالیت اول.
pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>(
        "SELECT * FROM user_sessions WHERE user_id = $1 ORDER BY last_seen_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// خروج از یک دستگاه مشخص با شناسه‌ی عمومی (فقط نشستِ متعلق به همان کاربر).
pub async fn revoke(db: &Db, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_sessions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// توکن یک نشست بر اساس شناسه‌ی عمومی (برای تشخیص «این دستگاه» هنگام لغو).
pub async fn token_of(db: &Db, id: Uuid, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT token FROM user_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// خروج از همه‌ی دستگاه‌ها به‌جز نشست فعلی.
pub async fn revoke_others(db: &Db, user_id: Uuid, keep: &str) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND token <> $2")
        .bind(user_id)
        .bind(keep)
        .execute(db)
        .await?;
    Ok(r.rows_affected())
}

/// حذف یک نشست (هنگام خروج عادی) — بدون قید کاربر.
pub async fn delete(db: &Db, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_sessions WHERE token = $1")
        .bind(token)
        .execute(db)
        .await?;
    Ok(())
}

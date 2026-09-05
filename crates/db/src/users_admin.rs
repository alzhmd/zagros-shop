//! Repository مدیریت کاربران (پنل ادمین) — فهرست، جستجو، فعال/مسدود.

use crate::accounts::User;
use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// ردیف فهرست کاربر با شمار سفارش.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub mobile: String,
    pub full_name: Option<String>,
    pub role: String,
    pub points: i64,
    pub is_active: bool,
    pub telegram_id: Option<i64>,
    pub order_count: i64,
    pub created_at: DateTime<Utc>,
}

impl AdminUserRow {
    pub fn role_fa(&self) -> &'static str {
        role_label(&self.role)
    }
    pub fn telegram_linked(&self) -> bool {
        self.telegram_id.is_some()
    }
}

pub fn role_label(role: &str) -> &'static str {
    match role {
        "admin" => "مدیر",
        "operator" => "اپراتور",
        "technician" => "تکنسین",
        _ => "مشتری",
    }
}

/// فهرست کاربران با جستجوی اختیاری روی موبایل/نام (آخرین‌ها اول).
pub async fn list(db: &Db, search: Option<&str>) -> Result<Vec<AdminUserRow>, sqlx::Error> {
    let base = r#"
        SELECT u.id, u.mobile, u.full_name, u.role, u.points, u.is_active, u.telegram_id,
               (SELECT count(*) FROM orders o WHERE o.user_id = u.id) AS order_count,
               u.created_at
        FROM users u
    "#;
    match search.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(q) => {
            sqlx::query_as::<_, AdminUserRow>(&format!(
                "{base} WHERE u.mobile ILIKE $1 OR u.full_name ILIKE $1
                 ORDER BY u.created_at DESC LIMIT 200"
            ))
            .bind(format!("%{q}%"))
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminUserRow>(&format!(
                "{base} ORDER BY u.created_at DESC LIMIT 200"
            ))
            .fetch_all(db)
            .await
        }
    }
}

pub async fn count(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
        .fetch_one(db)
        .await
}

/// فعال/مسدودکردن کاربر. مدیرها قابل‌مسدودشدن نیستند (محافظت).
pub async fn set_active(db: &Db, id: Uuid, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET is_active = $2 WHERE id = $1 AND role <> 'admin'")
        .bind(id)
        .bind(active)
        .execute(db)
        .await?;
    Ok(())
}

/// تغییر نقش کاربر.
pub async fn set_role(db: &Db, id: Uuid, role: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET role = $2 WHERE id = $1")
        .bind(id)
        .bind(role)
        .execute(db)
        .await?;
    Ok(())
}

/// شمار سفارش‌ها و درخواست‌های تعمیر یک کاربر (برای صفحه‌ی جزئیات).
pub async fn user_stats(db: &Db, user_id: Uuid) -> Result<(i64, i64), sqlx::Error> {
    let orders = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM orders WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;
    let repairs =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM repair_requests WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await?;
    Ok((orders, repairs))
}

/// یافتن کاربر خام بر اساس id (بدون شرط فعال‌بودن — برای پنل ادمین).
pub async fn find(db: &Db, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

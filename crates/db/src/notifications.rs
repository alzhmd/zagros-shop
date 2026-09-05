//! Repository اعلان‌ها + هشدار موجودشدن کالا.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub url: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn is_unread(&self) -> bool {
        self.read_at.is_none()
    }
    pub fn icon(&self) -> &'static str {
        match self.kind.as_str() {
            "order" => "📦",
            "review" => "⭐",
            "coupon" => "🎟",
            "club" => "🏅",
            "stock" => "✅",
            "price" => "🔻",
            _ => "🔔",
        }
    }
}

/// ساخت یک اعلان برای یک کاربر (بی‌صدا در صورت خطا توسط فراخوان مدیریت می‌شود).
pub async fn create(
    db: &Db,
    user_id: Uuid,
    kind: &str,
    title: &str,
    body: Option<&str>,
    url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, url) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(kind)
    .bind(title)
    .bind(body)
    .bind(url)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn unread_count(db: &Db, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

pub async fn list_for_user(db: &Db, user_id: Uuid, limit: i64) -> Result<Vec<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        "SELECT * FROM notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
}

/// همه‌ی اعلان‌های کاربر را خوانده‌شده علامت می‌زند (هنگام بازکردن صفحه).
pub async fn mark_all_read(db: &Db, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// پخش یک اعلان برای همه‌ی مشتریان (برای کمپین/کد تخفیف همگانی از پنل ادمین).
pub async fn broadcast(
    db: &Db,
    kind: &str,
    title: &str,
    body: Option<&str>,
    url: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let n = sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, url)
         SELECT id, $1, $2, $3, $4 FROM users WHERE is_active AND role = 'customer'",
    )
    .bind(kind)
    .bind(title)
    .bind(body)
    .bind(url)
    .execute(db)
    .await?
    .rows_affected();
    Ok(n)
}

// ===================== هشدار موجودشدن =====================

/// ثبت هشدار موجودشدن برای یک تنوع (idempotent).
pub async fn subscribe_stock(db: &Db, user_id: Uuid, variant_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO stock_alerts (user_id, variant_id) VALUES ($1,$2)
         ON CONFLICT (user_id, variant_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(variant_id)
    .execute(db)
    .await?;
    Ok(())
}

/// هنگام موجودشدن یک تنوع: به همه‌ی مشترکانِ منتظر اعلان می‌دهد و هشدارشان را می‌بندد.
/// مقدار برگشتی: تعداد اعلان ارسال‌شده.
pub async fn notify_back_in_stock(
    db: &Db,
    variant_id: Uuid,
    product_name: &str,
    product_slug: &str,
) -> Result<u64, sqlx::Error> {
    let url = format!("/product/{product_slug}");
    let title = format!("«{product_name}» دوباره موجود شد");
    let n = sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, url)
         SELECT user_id, 'stock', $2, NULL, $3 FROM stock_alerts
         WHERE variant_id = $1 AND notified_at IS NULL",
    )
    .bind(variant_id)
    .bind(&title)
    .bind(&url)
    .execute(db)
    .await?
    .rows_affected();
    sqlx::query("UPDATE stock_alerts SET notified_at = now() WHERE variant_id = $1 AND notified_at IS NULL")
        .bind(variant_id)
        .execute(db)
        .await?;
    Ok(n)
}

/// اعلان کاهش قیمت به کاربرانی که محصول را در علاقه‌مندی دارند.
pub async fn notify_price_drop(
    db: &Db,
    product_id: Uuid,
    product_name: &str,
    product_slug: &str,
) -> Result<u64, sqlx::Error> {
    let url = format!("/product/{product_slug}");
    let title = format!("قیمت «{product_name}» کاهش یافت");
    let n = sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, url)
         SELECT user_id, 'price', $2, NULL, $3 FROM wishlists WHERE product_id = $1",
    )
    .bind(product_id)
    .bind(&title)
    .bind(&url)
    .execute(db)
    .await?
    .rows_affected();
    Ok(n)
}

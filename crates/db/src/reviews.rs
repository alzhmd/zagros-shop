//! Repository نقد و امتیاز محصول (فاز ارتقا).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Review {
    pub id: Uuid,
    pub product_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub rating: i32,
    pub title: Option<String>,
    pub body: String,
    pub is_verified: bool,
    pub status: String,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
}

impl Review {
    pub fn stars_full(&self) -> usize {
        self.rating.clamp(0, 5) as usize
    }
    pub fn stars_empty(&self) -> usize {
        5 - self.stars_full()
    }
    /// رشته‌ی ستاره: پر (★) + خالی (☆).
    pub fn stars_str(&self) -> String {
        "★".repeat(self.stars_full()) + &"☆".repeat(self.stars_empty())
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
    pub fn helpful_fa(&self) -> String {
        domain::fa_digits(&self.helpful_count.to_string())
    }
    pub fn status_fa(&self) -> &'static str {
        match self.status.as_str() {
            "approved" => "تأییدشده",
            "rejected" => "ردشده",
            _ => "در انتظار",
        }
    }
}

/// خلاصه‌ی امتیاز یک محصول: میانگین و تعداد نقدهای تأییدشده.
#[derive(Debug, Clone, Default)]
pub struct RatingSummary {
    pub count: i64,
    pub average: f64,
}

impl RatingSummary {
    pub fn has_any(&self) -> bool {
        self.count > 0
    }
    pub fn average_fa(&self) -> String {
        domain::fa_digits(&format!("{:.1}", self.average))
    }
    /// میانگین با یک رقم اعشار به‌صورت ASCII (برای JSON-LD).
    pub fn average_1dp(&self) -> String {
        format!("{:.1}", self.average)
    }
    pub fn count_fa(&self) -> String {
        domain::fa_digits(&self.count.to_string())
    }
    /// تعداد ستاره‌ی پر (گرد‌شده) برای نمایش.
    pub fn stars_full(&self) -> usize {
        self.average.round().clamp(0.0, 5.0) as usize
    }
    pub fn stars_empty(&self) -> usize {
        5 - self.stars_full()
    }
    pub fn stars_str(&self) -> String {
        "★".repeat(self.stars_full()) + &"☆".repeat(self.stars_empty())
    }
}

pub struct NewReview {
    pub product_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub rating: i32,
    pub title: Option<String>,
    pub body: String,
    pub is_verified: bool,
}

/// ثبت یا به‌روزرسانی نقد یک کاربر (هر کاربر یک نقد فعال برای هر محصول).
pub async fn upsert(db: &Db, r: &NewReview) -> Result<(), sqlx::Error> {
    if r.user_id.is_some() {
        sqlx::query(
            r#"INSERT INTO product_reviews (product_id, user_id, author_name, rating, title, body, is_verified, status)
               VALUES ($1,$2,$3,$4,$5,$6,$7,'pending')
               ON CONFLICT (product_id, user_id) WHERE user_id IS NOT NULL
               DO UPDATE SET rating=EXCLUDED.rating, title=EXCLUDED.title, body=EXCLUDED.body,
                             author_name=EXCLUDED.author_name, status='pending', created_at=now()"#,
        )
        .bind(r.product_id)
        .bind(r.user_id)
        .bind(&r.author_name)
        .bind(r.rating)
        .bind(&r.title)
        .bind(&r.body)
        .bind(r.is_verified)
        .execute(db)
        .await?;
    } else {
        sqlx::query(
            r#"INSERT INTO product_reviews (product_id, user_id, author_name, rating, title, body, is_verified, status)
               VALUES ($1,NULL,$2,$3,$4,$5,$6,'pending')"#,
        )
        .bind(r.product_id)
        .bind(&r.author_name)
        .bind(r.rating)
        .bind(&r.title)
        .bind(&r.body)
        .bind(r.is_verified)
        .execute(db)
        .await?;
    }
    Ok(())
}

/// نقدهای تأییدشده‌ی یک محصول (برای نمایش عمومی).
pub async fn approved_for_product(db: &Db, product_id: Uuid) -> Result<Vec<Review>, sqlx::Error> {
    sqlx::query_as::<_, Review>(
        "SELECT * FROM product_reviews WHERE product_id = $1 AND status = 'approved'
         ORDER BY created_at DESC",
    )
    .bind(product_id)
    .fetch_all(db)
    .await
}

/// خلاصه‌ی امتیاز یک محصول (فقط نقدهای تأییدشده).
pub async fn summary(db: &Db, product_id: Uuid) -> Result<RatingSummary, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, Option<f64>)>(
        "SELECT count(*)::bigint, avg(rating)::float8 FROM product_reviews
         WHERE product_id = $1 AND status = 'approved'",
    )
    .bind(product_id)
    .fetch_one(db)
    .await?;
    Ok(RatingSummary {
        count: row.0,
        average: row.1.unwrap_or(0.0),
    })
}

/// آیا کاربر این محصول را خریده و تحویل گرفته/پرداخته؟ (برای نشان «خرید تأییدشده».)
pub async fn has_purchased(db: &Db, user_id: Uuid, product_id: Uuid) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         JOIN product_variants v ON v.id = oi.variant_id
         WHERE o.user_id = $1 AND v.product_id = $2",
    )
    .bind(user_id)
    .bind(product_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

// ---------- مرکز دیدگاه‌های کاربر ----------

/// یک ردیف نقدِ کاربر برای صفحه‌ی «دیدگاه‌های من» (با نام/نامک/تصویر محصول).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MyReviewRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_slug: String,
    pub primary_image: Option<String>,
    pub rating: i32,
    pub title: Option<String>,
    pub body: String,
    pub is_verified: bool,
    pub status: String,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
}

impl MyReviewRow {
    pub fn stars_full(&self) -> usize {
        self.rating.clamp(0, 5) as usize
    }
    pub fn stars_empty(&self) -> usize {
        5 - self.stars_full()
    }
    pub fn stars_str(&self) -> String {
        "★".repeat(self.stars_full()) + &"☆".repeat(self.stars_empty())
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
    pub fn helpful_fa(&self) -> String {
        domain::fa_digits(&self.helpful_count.to_string())
    }
    pub fn status_fa(&self) -> &'static str {
        match self.status.as_str() {
            "approved" => "تأییدشده",
            "rejected" => "ردشده",
            _ => "در انتظار بررسی",
        }
    }
    /// کلید وضعیت برای رنگ‌بندی (approved | rejected | pending).
    pub fn status_key(&self) -> &str {
        match self.status.as_str() {
            "approved" | "rejected" => &self.status,
            _ => "pending",
        }
    }
}

/// همه‌ی نقدهای یک کاربر (هر وضعیتی) — جدیدترین اول.
pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<MyReviewRow>, sqlx::Error> {
    sqlx::query_as::<_, MyReviewRow>(
        r#"
        SELECT r.id, r.product_id, p.name AS product_name, p.slug AS product_slug,
               (SELECT path FROM product_images pi WHERE pi.product_id = p.id
                ORDER BY pi.is_primary DESC, pi.sort_order LIMIT 1) AS primary_image,
               r.rating, r.title, r.body, r.is_verified, r.status, r.helpful_count, r.created_at
        FROM product_reviews r
        JOIN products p ON p.id = r.product_id
        WHERE r.user_id = $1
        ORDER BY r.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// ثبت رأی «مفید بود» (هر کاربر یک‌بار) و بازگرداندن شمارِ به‌روزِ مفید.
pub async fn mark_helpful(db: &Db, review_id: Uuid, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let inserted = sqlx::query(
        "INSERT INTO review_helpful (review_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(review_id)
    .bind(user_id)
    .execute(db)
    .await?
    .rows_affected();
    if inserted > 0 {
        sqlx::query("UPDATE product_reviews SET helpful_count = helpful_count + 1 WHERE id = $1")
            .bind(review_id)
            .execute(db)
            .await?;
    }
    sqlx::query_scalar::<_, i32>("SELECT helpful_count FROM product_reviews WHERE id = $1")
        .bind(review_id)
        .fetch_optional(db)
        .await
        .map(|o| o.unwrap_or(0) as i64)
}

// ---------- مدیریت (ادمین) ----------

/// ردیف نقد برای پنل ادمین — با نام محصول.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdminReviewRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_slug: String,
    pub author_name: String,
    pub rating: i32,
    pub title: Option<String>,
    pub body: String,
    pub is_verified: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl AdminReviewRow {
    pub fn stars_full(&self) -> usize {
        self.rating.clamp(0, 5) as usize
    }
    pub fn stars_empty(&self) -> usize {
        5 - self.stars_full()
    }
    pub fn stars_str(&self) -> String {
        "★".repeat(self.stars_full()) + &"☆".repeat(self.stars_empty())
    }
    pub fn status_fa(&self) -> &'static str {
        match self.status.as_str() {
            "approved" => "تأییدشده",
            "rejected" => "ردشده",
            _ => "در انتظار",
        }
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
}

pub async fn list_all(db: &Db, status_filter: Option<&str>) -> Result<Vec<AdminReviewRow>, sqlx::Error> {
    let base = r#"
        SELECT r.id, r.product_id, p.name AS product_name, p.slug AS product_slug,
               r.author_name, r.rating, r.title, r.body, r.is_verified, r.status, r.created_at
        FROM product_reviews r
        JOIN products p ON p.id = r.product_id
    "#;
    match status_filter {
        Some(s) => {
            sqlx::query_as::<_, AdminReviewRow>(&format!(
                "{base} WHERE r.status = $1 ORDER BY r.created_at DESC LIMIT 300"
            ))
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminReviewRow>(&format!(
                "{base} ORDER BY r.created_at DESC LIMIT 300"
            ))
            .fetch_all(db)
            .await
        }
    }
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE product_reviews SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// اطلاعاتِ لازم برای اعلانِ نقد: (user_id, نام محصول, نامک) — برای اطلاع‌رسانی پس از تأیید.
pub async fn notify_info(
    db: &Db,
    id: Uuid,
) -> Result<Option<(Option<Uuid>, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (Option<Uuid>, String, String)>(
        "SELECT r.user_id, p.name, p.slug FROM product_reviews r
         JOIN products p ON p.id = r.product_id WHERE r.id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM product_reviews WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn count_pending(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM product_reviews WHERE status = 'pending'")
        .fetch_one(db)
        .await
}

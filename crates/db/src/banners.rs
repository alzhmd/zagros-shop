//! Repository بنر/کمپین — اسلایدر صفحه‌ی اصلی و بنرهای جایگاهی.

use crate::Db;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Banner {
    pub id: Uuid,
    pub placement: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub image_path: Option<String>,
    pub link: Option<String>,
    pub cta_label: Option<String>,
    pub bg_color: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// بنرهای فعالِ یک جایگاه برای نمایش عمومی (با احترام به بازه‌ی زمانی).
pub async fn active_for(db: &Db, placement: &str) -> Result<Vec<Banner>, sqlx::Error> {
    sqlx::query_as::<_, Banner>(
        "SELECT * FROM banners
         WHERE is_active AND placement = $1
           AND (starts_at IS NULL OR starts_at <= now())
           AND (ends_at   IS NULL OR ends_at   >= now())
         ORDER BY sort_order, created_at",
    )
    .bind(placement)
    .fetch_all(db)
    .await
}

/// همه‌ی بنرها برای پنل ادمین (بدون فیلتر).
pub async fn list_all(db: &Db) -> Result<Vec<Banner>, sqlx::Error> {
    sqlx::query_as::<_, Banner>("SELECT * FROM banners ORDER BY placement, sort_order, created_at")
        .fetch_all(db)
        .await
}

pub async fn find(db: &Db, id: Uuid) -> Result<Option<Banner>, sqlx::Error> {
    sqlx::query_as::<_, Banner>("SELECT * FROM banners WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub struct BannerInput {
    pub placement: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub image_path: Option<String>,
    pub link: Option<String>,
    pub cta_label: Option<String>,
    pub bg_color: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

pub async fn create(db: &Db, b: &BannerInput) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO banners
           (placement, title, subtitle, image_path, link, cta_label, bg_color, sort_order, is_active)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(&b.placement)
    .bind(&b.title)
    .bind(&b.subtitle)
    .bind(&b.image_path)
    .bind(&b.link)
    .bind(&b.cta_label)
    .bind(&b.bg_color)
    .bind(b.sort_order)
    .bind(b.is_active)
    .fetch_one(db)
    .await
}

pub async fn update(db: &Db, id: Uuid, b: &BannerInput) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE banners SET
           placement=$2, title=$3, subtitle=$4, image_path=$5, link=$6,
           cta_label=$7, bg_color=$8, sort_order=$9, is_active=$10
         WHERE id=$1",
    )
    .bind(id)
    .bind(&b.placement)
    .bind(&b.title)
    .bind(&b.subtitle)
    .bind(&b.image_path)
    .bind(&b.link)
    .bind(&b.cta_label)
    .bind(&b.bg_color)
    .bind(b.sort_order)
    .bind(b.is_active)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM banners WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

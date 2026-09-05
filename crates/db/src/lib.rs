//! لایه‌ی دسترسی به داده — اتصال Postgres، migrationها، و repositoryها.

pub mod accounts;
pub mod attributes;
pub mod banners;
pub mod cart;
pub mod catalog;
pub mod catalog_admin;
pub mod coupons;
pub mod gift_cards;
pub mod loyalty;
pub mod models;
pub mod notifications;
pub mod orders;
pub mod orders_admin;
pub mod qa;
pub mod repairs;
pub mod returns;
pub mod reviews;
pub mod sessions;
pub mod settings;
pub mod users_admin;
pub mod support;
pub mod wallet;
pub mod wishlist;

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub type Db = PgPool;

/// ساخت connection pool با تنظیمات بهینه برای سرور کم‌منبع (تک‌هسته).
pub async fn connect(database_url: &str) -> Result<Db, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// اجرای migrationهای داخل پوشه‌ی `migrations/` در ریشه‌ی workspace.
pub async fn migrate(pool: &Db) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

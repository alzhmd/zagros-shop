//! Repository علاقه‌مندی‌ها (Wishlist) — فاز ارتقا.

use crate::models::ProductCard;
use crate::Db;
use uuid::Uuid;

/// افزودن/حذف یک محصول از علاقه‌مندی‌ها (toggle). مقدار برگشتی: وضعیت جدید (true = اکنون در لیست).
pub async fn toggle(db: &Db, user_id: Uuid, product_id: Uuid) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query("DELETE FROM wishlists WHERE user_id = $1 AND product_id = $2")
        .bind(user_id)
        .bind(product_id)
        .execute(db)
        .await?
        .rows_affected();
    if deleted > 0 {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO wishlists (user_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(product_id)
    .execute(db)
    .await?;
    Ok(true)
}

pub async fn contains(db: &Db, user_id: Uuid, product_id: Uuid) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM wishlists WHERE user_id = $1 AND product_id = $2",
    )
    .bind(user_id)
    .bind(product_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

/// محصولات علاقه‌مندی کاربر (کارت‌ها برای نمایش).
pub async fn list_cards(db: &Db, user_id: Uuid) -> Result<Vec<ProductCard>, sqlx::Error> {
    sqlx::query_as::<_, ProductCard>(
        r#"
        SELECT p.id, p.name, p.slug, p.short_desc, p.base_price,
               b.name AS brand_name, c.slug AS category_slug,
               (SELECT path FROM product_images pi WHERE pi.product_id = p.id
                ORDER BY pi.is_primary DESC, pi.sort_order LIMIT 1) AS primary_image,
               EXISTS (SELECT 1 FROM product_variants v
                       WHERE v.product_id = p.id AND v.is_active AND v.stock > 0) AS in_stock,
               COALESCE((SELECT avg(rating) FROM product_reviews r
                         WHERE r.product_id = p.id AND r.status = 'approved'), 0)::float8 AS rating_avg,
               (SELECT count(*) FROM product_reviews r
                WHERE r.product_id = p.id AND r.status = 'approved') AS rating_count
        FROM wishlists w
        JOIN products p ON p.id = w.product_id
        LEFT JOIN brands b ON b.id = p.brand_id
        LEFT JOIN categories c ON c.id = p.category_id
        WHERE w.user_id = $1 AND p.is_active
        ORDER BY w.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn count(db: &Db, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM wishlists WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
}

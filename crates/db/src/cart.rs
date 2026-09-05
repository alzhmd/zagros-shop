//! Repository سبد خرید.

use crate::Db;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CartItemView {
    pub item_id: Uuid,
    pub variant_id: Uuid,
    pub product_slug: String,
    pub product_name: String,
    pub variant_title: String,
    pub unit_price: i64,
    pub compare_at_price: Option<i64>,
    pub qty: i32,
    pub stock: i32,
    pub image: Option<String>,
}

impl CartItemView {
    pub fn line_total(&self) -> i64 {
        self.unit_price * self.qty as i64
    }
    pub fn line_total_fa(&self) -> String {
        domain::Rial(self.line_total()).format_toman_fa()
    }
    pub fn unit_price_fa(&self) -> String {
        domain::Rial(self.unit_price).format_toman_fa()
    }
    /// آیا این قلم تخفیف دارد؟
    pub fn has_discount(&self) -> bool {
        self.compare_at_price.map_or(false, |c| c > self.unit_price)
    }
    /// مجموع قیمت قبل از تخفیف برای این خط (برای نمایش پس‌انداز).
    pub fn compare_line_fa(&self) -> String {
        domain::Rial(self.compare_at_price.unwrap_or(0) * self.qty as i64).format_toman_fa()
    }
}

/// یافتن سبد موجود برای یک session key (بدون ساخت). برای شمارش سبک در هدر.
pub async fn find_cart(db: &Db, session_key: &str) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM carts WHERE session_key = $1")
        .bind(session_key)
        .fetch_optional(db)
        .await
}

/// یافتن یا ساخت سبد برای یک session key (مهمان یا کاربر).
pub async fn get_or_create_cart(db: &Db, session_key: &str) -> Result<Uuid, sqlx::Error> {
    if let Some(id) =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM carts WHERE session_key = $1")
            .bind(session_key)
            .fetch_optional(db)
            .await?
    {
        return Ok(id);
    }
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO carts (session_key) VALUES ($1) RETURNING id",
    )
    .bind(session_key)
    .fetch_one(db)
    .await
}

/// افزودن یا افزایش یک قلم در سبد.
pub async fn add_item(
    db: &Db,
    cart_id: Uuid,
    variant_id: Uuid,
    qty: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO cart_items (cart_id, variant_id, qty) VALUES ($1, $2, $3)
         ON CONFLICT (cart_id, variant_id) DO UPDATE SET qty = cart_items.qty + EXCLUDED.qty",
    )
    .bind(cart_id)
    .bind(variant_id)
    .bind(qty)
    .execute(db)
    .await?;
    Ok(())
}

/// تغییر تعداد یک قلم (۰ یا کمتر = حذف).
pub async fn set_qty(db: &Db, cart_id: Uuid, item_id: Uuid, qty: i32) -> Result<(), sqlx::Error> {
    if qty <= 0 {
        sqlx::query("DELETE FROM cart_items WHERE id = $1 AND cart_id = $2")
            .bind(item_id)
            .bind(cart_id)
            .execute(db)
            .await?;
    } else {
        sqlx::query("UPDATE cart_items SET qty = $1 WHERE id = $2 AND cart_id = $3")
            .bind(qty)
            .bind(item_id)
            .bind(cart_id)
            .execute(db)
            .await?;
    }
    Ok(())
}

pub async fn remove_item(db: &Db, cart_id: Uuid, item_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM cart_items WHERE id = $1 AND cart_id = $2")
        .bind(item_id)
        .bind(cart_id)
        .execute(db)
        .await?;
    Ok(())
}

const ITEM_SELECT: &str = r#"
    SELECT ci.id AS item_id, ci.variant_id, p.slug AS product_slug,
           p.name AS product_name, v.title AS variant_title,
           v.price AS unit_price, v.compare_at_price, ci.qty, v.stock,
           (SELECT path FROM product_images pi WHERE pi.product_id = p.id
            ORDER BY pi.is_primary DESC, pi.sort_order LIMIT 1) AS image
    FROM cart_items ci
    JOIN product_variants v ON v.id = ci.variant_id
    JOIN products p ON p.id = v.product_id
"#;

/// اقلام فعال سبد (به‌جز «ذخیره برای بعد»).
pub async fn items(db: &Db, cart_id: Uuid) -> Result<Vec<CartItemView>, sqlx::Error> {
    sqlx::query_as::<_, CartItemView>(&format!(
        "{ITEM_SELECT} WHERE ci.cart_id = $1 AND NOT ci.saved_for_later ORDER BY p.name"
    ))
    .bind(cart_id)
    .fetch_all(db)
    .await
}

/// اقلامِ «ذخیره برای بعد».
pub async fn saved_items(db: &Db, cart_id: Uuid) -> Result<Vec<CartItemView>, sqlx::Error> {
    sqlx::query_as::<_, CartItemView>(&format!(
        "{ITEM_SELECT} WHERE ci.cart_id = $1 AND ci.saved_for_later ORDER BY p.name"
    ))
    .bind(cart_id)
    .fetch_all(db)
    .await
}

/// تغییر وضعیت «ذخیره برای بعد» یک قلم.
pub async fn set_saved(db: &Db, cart_id: Uuid, item_id: Uuid, saved: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE cart_items SET saved_for_later = $1 WHERE id = $2 AND cart_id = $3")
        .bind(saved)
        .bind(item_id)
        .bind(cart_id)
        .execute(db)
        .await?;
    Ok(())
}

/// تعداد کل اقلام سبد (برای نشانگر هدر).
pub async fn item_count(db: &Db, cart_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, Option<i64>>(
        "SELECT sum(qty)::bigint FROM cart_items WHERE cart_id = $1",
    )
    .bind(cart_id)
    .fetch_one(db)
    .await
    .map(|o| o.unwrap_or(0))
}

pub async fn clear(db: &Db, cart_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM cart_items WHERE cart_id = $1")
        .bind(cart_id)
        .execute(db)
        .await?;
    Ok(())
}

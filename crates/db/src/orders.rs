//! Repository سفارش و پرداخت — شامل تسویه‌ی تراکنشی.

use crate::cart::CartItemView;
use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Order {
    pub id: Uuid,
    pub order_number: i64,
    pub user_id: Option<Uuid>,
    pub status: String,
    pub payment_method: String,
    pub subtotal: i64,
    pub discount: i64,
    pub shipping_cost: i64,
    pub total: i64,
    pub ship_receiver: Option<String>,
    pub ship_phone: Option<String>,
    pub ship_province: Option<String>,
    pub ship_city: Option<String>,
    pub ship_address: Option<String>,
    pub shipping_method: Option<String>,
    pub coupon_code: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Order {
    pub fn total_fa(&self) -> String {
        domain::Rial(self.total).format_toman_fa()
    }
    pub fn subtotal_fa(&self) -> String {
        domain::Rial(self.subtotal).format_toman_fa()
    }
    pub fn discount_fa(&self) -> String {
        domain::Rial(self.discount).format_toman_fa()
    }
    pub fn shipping_fa(&self) -> String {
        domain::Rial(self.shipping_cost).format_toman_fa()
    }
    pub fn status_fa(&self) -> &'static str {
        status_label(&self.status)
    }

    /// شاخص مرحله‌ی فعلی روی نوار پیشرفت (۰..۴)؛ برای «لغوشده» ‎-۱.
    /// مراحل: ۰ ثبت · ۱ پرداخت · ۲ آماده‌سازی · ۳ ارسال · ۴ تحویل.
    pub fn step(&self) -> i32 {
        match self.status.as_str() {
            "pending" | "awaiting_payment" => 0,
            "paid" | "cod_pending" => 1,
            "processing" => 2,
            "shipped" => 3,
            "delivered" => 4,
            "cancelled" => -1,
            _ => 0,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == "cancelled"
    }

    /// سفارش «در جریان» (نه تحویل‌شده و نه لغوشده) — برای بنر «نیازمند توجه».
    pub fn is_active(&self) -> bool {
        !matches!(self.status.as_str(), "delivered" | "cancelled")
    }

    /// گروه برای فیلتر صفحه‌ی سفارش‌ها.
    pub fn group(&self) -> &'static str {
        match self.status.as_str() {
            "delivered" => "delivered",
            "cancelled" => "cancelled",
            _ => "active",
        }
    }

    pub fn can_invoice(&self) -> bool {
        matches!(
            self.status.as_str(),
            "paid" | "processing" | "shipped" | "delivered" | "cod_pending"
        )
    }

    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status.as_str(),
            "pending" | "awaiting_payment" | "paid" | "cod_pending"
        )
    }

    pub fn can_return(&self) -> bool {
        self.status == "delivered"
    }

    /// زمانِ نسبیِ ثبت سفارش به فارسی («همین حالا»، «دیروز»، «۳ روز پیش»…).
    pub fn created_relative_fa(&self) -> String {
        let d = Utc::now().signed_duration_since(self.created_at);
        let mins = d.num_minutes();
        let hours = d.num_hours();
        let days = d.num_days();
        let s = if mins < 1 {
            "همین حالا".to_string()
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
        fa_digits(&s)
    }
}

/// تبدیل ارقام انگلیسی رشته به فارسی (۰۱۲۳۴۵۶۷۸۹).
fn fa_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32(0x06F0 + (c as u32 - '0' as u32)).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// برچسب فارسی وضعیت سفارش — مشترک بین Order و ردیف‌های ادمین.
pub fn status_label(s: &str) -> &'static str {
    match s {
        "pending" => "در انتظار",
        "awaiting_payment" => "در انتظار پرداخت",
        "paid" => "پرداخت‌شده",
        "processing" => "در حال پردازش",
        "shipped" => "ارسال‌شده",
        "delivered" => "تحویل‌شده",
        "cancelled" => "لغو‌شده",
        "cod_pending" => "پرداخت در محل",
        _ => "نامشخص",
    }
}

/// همه‌ی وضعیت‌های معتبر سفارش (برای فیلتر و منوی تغییر وضعیت).
pub const ORDER_STATUSES: [&str; 8] = [
    "pending",
    "awaiting_payment",
    "paid",
    "processing",
    "shipped",
    "delivered",
    "cancelled",
    "cod_pending",
];

pub struct ShippingInfo {
    pub receiver: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub address: String,
}

/// تخفیف اعمال‌شده روی سفارش (از کوپن).
#[derive(Debug, Clone, Default)]
pub struct OrderDiscount {
    pub amount: i64,
    pub coupon_code: Option<String>,
}

/// ساخت سفارش از روی سبد، با کاهش موجودی، در یک تراکنش اتمیک.
#[allow(clippy::too_many_arguments)] // یک insert اتمیک؛ struct پارامتر فقط فهرست را جابه‌جا می‌کرد
pub async fn create_from_cart(
    db: &Db,
    user_id: Option<Uuid>,
    items: &[CartItemView],
    ship: &ShippingInfo,
    payment_method: &str,
    shipping_cost: i64,
    shipping_method: Option<&str>,
    discount: &OrderDiscount,
) -> Result<Order, sqlx::Error> {
    let mut tx = db.begin().await?;

    let subtotal: i64 = items.iter().map(|i| i.line_total()).sum();
    // تخفیف هرگز بیش از جمع اقلام نمی‌شود.
    let disc = discount.amount.clamp(0, subtotal);
    let total = subtotal - disc + shipping_cost;
    let status = if payment_method == "cod" {
        "cod_pending"
    } else {
        "awaiting_payment"
    };

    let order = sqlx::query_as::<_, Order>(
        r#"INSERT INTO orders
           (user_id, status, payment_method, subtotal, discount, shipping_cost, total,
            ship_receiver, ship_phone, ship_province, ship_city, ship_address, shipping_method, coupon_code)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING *"#,
    )
    .bind(user_id)
    .bind(status)
    .bind(payment_method)
    .bind(subtotal)
    .bind(disc)
    .bind(shipping_cost)
    .bind(total)
    .bind(&ship.receiver)
    .bind(&ship.phone)
    .bind(&ship.province)
    .bind(&ship.city)
    .bind(&ship.address)
    .bind(shipping_method)
    .bind(&discount.coupon_code)
    .fetch_one(&mut *tx)
    .await?;

    for it in items {
        // کاهش موجودی با بررسی کفایت (اتمیک).
        let affected = sqlx::query(
            "UPDATE product_variants SET stock = stock - $1 WHERE id = $2 AND stock >= $1",
        )
        .bind(it.qty)
        .bind(it.variant_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol(format!(
                "موجودی ناکافی برای «{}»",
                it.product_name
            )));
        }

        sqlx::query(
            r#"INSERT INTO order_items
               (order_id, variant_id, product_name, variant_title, unit_price, qty, line_total)
               VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
        )
        .bind(order.id)
        .bind(it.variant_id)
        .bind(&it.product_name)
        .bind(&it.variant_title)
        .bind(it.unit_price)
        .bind(it.qty)
        .bind(it.line_total())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(order)
}

pub async fn find(db: &Db, id: Uuid) -> Result<Option<Order>, sqlx::Error> {
    sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// یک قلم سفارش برای نمایش به مشتری (snapshot خرید + slug برای لینک).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OrderLine {
    pub product_name: String,
    pub variant_title: Option<String>,
    pub unit_price: i64,
    pub qty: i32,
    pub line_total: i64,
    pub product_slug: Option<String>,
}

impl OrderLine {
    pub fn unit_price_fa(&self) -> String {
        domain::Rial(self.unit_price).format_toman_fa()
    }
    pub fn line_total_fa(&self) -> String {
        domain::Rial(self.line_total).format_toman_fa()
    }
}

/// اقلام یک سفارش (برای صفحه‌ی جزئیات مشتری). slug محصول در صورت وجود.
pub async fn items_for(db: &Db, order_id: Uuid) -> Result<Vec<OrderLine>, sqlx::Error> {
    sqlx::query_as::<_, OrderLine>(
        "SELECT oi.product_name, oi.variant_title, oi.unit_price, oi.qty, oi.line_total,
                p.slug AS product_slug
         FROM order_items oi
         LEFT JOIN product_variants v ON v.id = oi.variant_id
         LEFT JOIN products p ON p.id = v.product_id
         WHERE oi.order_id = $1 ORDER BY oi.product_name",
    )
    .bind(order_id)
    .fetch_all(db)
    .await
}

/// قلم سفارش با شناسه (برای فرم مرجوعی — انتخاب اقلام برگشتی).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OrderItemFull {
    pub id: Uuid,
    pub product_name: String,
    pub variant_title: Option<String>,
    pub unit_price: i64,
    pub qty: i32,
}
impl OrderItemFull {
    pub fn unit_price_fa(&self) -> String {
        domain::Rial(self.unit_price).format_toman_fa()
    }
}

pub async fn items_with_id(db: &Db, order_id: Uuid) -> Result<Vec<OrderItemFull>, sqlx::Error> {
    sqlx::query_as::<_, OrderItemFull>(
        "SELECT id, product_name, variant_title, unit_price, qty FROM order_items
         WHERE order_id = $1 ORDER BY product_name",
    )
    .bind(order_id)
    .fetch_all(db)
    .await
}

/// افزودن دوباره‌ی اقلام یک سفارش به سبد (سفارش مجدد). فقط variantهای فعالِ موجود.
/// مقدار برگشتی: تعداد اقلامی که اضافه شد.
pub async fn reorder_into_cart(db: &Db, order_id: Uuid, cart_id: Uuid) -> Result<i64, sqlx::Error> {
    let added = sqlx::query_scalar::<_, i64>(
        "WITH ins AS (
            INSERT INTO cart_items (cart_id, variant_id, qty)
            SELECT $2, oi.variant_id, oi.qty
            FROM order_items oi
            JOIN product_variants v ON v.id = oi.variant_id AND v.is_active
            WHERE oi.order_id = $1 AND oi.variant_id IS NOT NULL
            ON CONFLICT (cart_id, variant_id) DO UPDATE SET qty = cart_items.qty + EXCLUDED.qty
            RETURNING 1
         )
         SELECT count(*)::bigint FROM ins",
    )
    .bind(order_id)
    .bind(cart_id)
    .fetch_one(db)
    .await?;
    Ok(added)
}

pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<Order>, sqlx::Error> {
    sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE user_id = $1 ORDER BY created_at DESC")
        .bind(user_id)
        .fetch_all(db)
        .await
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_note(db: &Db, id: Uuid, note: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE orders SET note = $1 WHERE id = $2")
        .bind(note)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// لغو سفارش + بازگرداندن موجودی به انبار (اتمیک، idempotent).
/// فقط زمانی موجودی برمی‌گردد که سفارش قبلاً «cancelled» نبوده باشد —
/// تا از بازگرداندن دوباره‌ی موجودی جلوگیری شود.
pub async fn cancel_and_restock(db: &Db, id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = db.begin().await?;
    // فقط اگر هنوز لغو نشده، وضعیت را عوض کن (قفل ردیف).
    let changed = sqlx::query(
        "UPDATE orders SET status = 'cancelled' WHERE id = $1 AND status <> 'cancelled'",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if changed == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    // بازگرداندن موجودی هر قلم (variantهای حذف‌نشده).
    sqlx::query(
        "UPDATE product_variants v SET stock = v.stock + oi.qty
         FROM order_items oi
         WHERE oi.order_id = $1 AND oi.variant_id = v.id",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

// ---------- پرداخت ----------

#[derive(Debug, Clone, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub order_id: Uuid,
    pub provider: String,
    pub gateway_token: Option<String>,
    pub amount: i64,
    pub state: String,
    pub ref_id: Option<String>,
}

pub async fn create_payment(
    db: &Db,
    order_id: Uuid,
    provider: &str,
    amount: i64,
) -> Result<Payment, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        "INSERT INTO payments (order_id, provider, amount, state)
         VALUES ($1, $2, $3, 'created') RETURNING id, order_id, provider, gateway_token, amount, state, ref_id",
    )
    .bind(order_id)
    .bind(provider)
    .bind(amount)
    .fetch_one(db)
    .await
}

pub async fn set_payment_token(db: &Db, id: Uuid, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE payments SET gateway_token = $1, state = 'redirected' WHERE id = $2")
        .bind(token)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn find_payment_by_token(
    db: &Db,
    provider: &str,
    token: &str,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        "SELECT id, order_id, provider, gateway_token, amount, state, ref_id
         FROM payments WHERE provider = $1 AND gateway_token = $2",
    )
    .bind(provider)
    .bind(token)
    .fetch_optional(db)
    .await
}

/// ثبت موفقیت پرداخت به‌صورت idempotent (برای callback تکراری امن).
pub async fn mark_payment_verified(
    db: &Db,
    payment_id: Uuid,
    ref_id: &str,
    card_pan: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query(
        "UPDATE payments SET state = 'verified', ref_id = $1, card_pan = $2, verified_at = now()
         WHERE id = $3",
    )
    .bind(ref_id)
    .bind(card_pan)
    .bind(payment_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE orders SET status = 'paid'
         WHERE id = (SELECT order_id FROM payments WHERE id = $1) AND status != 'paid'",
    )
    .bind(payment_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

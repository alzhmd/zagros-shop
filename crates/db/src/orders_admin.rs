//! Repository مدیریت سفارش‌ها (پنل ادمین) — فهرست با فیلتر، و جزئیات کامل.

use crate::orders::Order;
use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// ردیف فهرست سفارش با اطلاعات مشتری (برای جدول ادمین).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdminOrderRow {
    pub id: Uuid,
    pub order_number: i64,
    pub status: String,
    pub payment_method: String,
    pub total: i64,
    pub user_id: Option<Uuid>,
    pub customer_mobile: Option<String>,
    pub ship_receiver: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AdminOrderRow {
    pub fn total_fa(&self) -> String {
        domain::Rial(self.total).format_toman_fa()
    }
    pub fn status_fa(&self) -> &'static str {
        crate::orders::status_label(&self.status)
    }
    pub fn is_online(&self) -> bool {
        self.payment_method != "cod"
    }
}

/// فهرست سفارش‌ها با فیلتر وضعیت اختیاری (آخرین‌ها اول).
pub async fn list(db: &Db, status: Option<&str>) -> Result<Vec<AdminOrderRow>, sqlx::Error> {
    let base = r#"
        SELECT o.id, o.order_number, o.status, o.payment_method, o.total,
               o.user_id, u.mobile AS customer_mobile, o.ship_receiver, o.created_at
        FROM orders o
        LEFT JOIN users u ON u.id = o.user_id
    "#;
    match status {
        Some(s) => {
            sqlx::query_as::<_, AdminOrderRow>(&format!(
                "{base} WHERE o.status = $1 ORDER BY o.created_at DESC LIMIT 200"
            ))
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminOrderRow>(&format!(
                "{base} ORDER BY o.created_at DESC LIMIT 200"
            ))
            .fetch_all(db)
            .await
        }
    }
}

/// شمارش سفارش‌ها بر اساس وضعیت (برای تب‌های فیلتر).
pub async fn count_by_status(db: &Db) -> Result<Vec<(String, i64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT status, count(*)::bigint FROM orders GROUP BY status",
    )
    .fetch_all(db)
    .await
}

/// یک قلم سفارش (snapshot لحظه‌ی خرید).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OrderItem {
    pub product_name: String,
    pub variant_title: Option<String>,
    pub unit_price: i64,
    pub qty: i32,
    pub line_total: i64,
}

impl OrderItem {
    pub fn unit_price_fa(&self) -> String {
        domain::Rial(self.unit_price).format_toman_fa()
    }
    pub fn line_total_fa(&self) -> String {
        domain::Rial(self.line_total).format_toman_fa()
    }
}

/// رکورد پرداخت (برای نمایش در جزئیات سفارش).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PaymentRow {
    pub provider: String,
    pub amount: i64,
    pub state: String,
    pub ref_id: Option<String>,
    pub card_pan: Option<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

impl PaymentRow {
    pub fn amount_fa(&self) -> String {
        domain::Rial(self.amount).format_toman_fa()
    }
    pub fn state_fa(&self) -> &'static str {
        match self.state.as_str() {
            "created" => "ایجادشده",
            "redirected" => "هدایت به درگاه",
            "paid" => "پرداخت‌شده",
            "verified" => "تأییدشده",
            "failed" => "ناموفق",
            _ => "نامشخص",
        }
    }
}

/// جزئیات کامل سفارش برای پنل ادمین.
pub struct OrderDetail {
    pub order: Order,
    pub items: Vec<OrderItem>,
    pub payments: Vec<PaymentRow>,
    pub customer_mobile: Option<String>,
}

pub async fn detail(db: &Db, id: Uuid) -> Result<Option<OrderDetail>, sqlx::Error> {
    let Some(order) = crate::orders::find(db, id).await? else {
        return Ok(None);
    };
    let items = sqlx::query_as::<_, OrderItem>(
        "SELECT product_name, variant_title, unit_price, qty, line_total
         FROM order_items WHERE order_id = $1 ORDER BY product_name",
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    let payments = sqlx::query_as::<_, PaymentRow>(
        "SELECT provider, amount, state, ref_id, card_pan, created_at, verified_at
         FROM payments WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    let customer_mobile = match order.user_id {
        Some(uid) => {
            sqlx::query_scalar::<_, String>("SELECT mobile FROM users WHERE id = $1")
                .bind(uid)
                .fetch_optional(db)
                .await?
        }
        None => None,
    };
    Ok(Some(OrderDetail {
        order,
        items,
        payments,
        customer_mobile,
    }))
}

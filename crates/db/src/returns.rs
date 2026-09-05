//! Repository مرجوعی/بازگشت کالا (Returns / Refunds) — فاز ارتقا.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Return {
    pub id: Uuid,
    pub return_number: i64,
    pub order_id: Uuid,
    pub user_id: Option<Uuid>,
    pub reason: String,
    pub description: Option<String>,
    pub status: String,
    pub refund_amount: Option<i64>,
    pub admin_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Return {
    pub fn status_fa(&self) -> &'static str {
        status_label(&self.status)
    }
    pub fn reason_fa(&self) -> &'static str {
        reason_label(&self.reason)
    }
    pub fn refund_fa(&self) -> Option<String> {
        self.refund_amount.map(|a| domain::Rial(a).format_toman_fa())
    }
    pub fn number_fa(&self) -> String {
        domain::fa_digits(&self.return_number.to_string())
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
    pub fn is_open(&self) -> bool {
        !matches!(self.status.as_str(), "refunded" | "rejected" | "cancelled")
    }
}

pub fn status_label(s: &str) -> &'static str {
    match s {
        "requested" => "ثبت‌شده (در انتظار بررسی)",
        "approved" => "تأییدشده — ارسال کالا به ما",
        "received" => "کالا دریافت شد",
        "refunded" => "بازپرداخت انجام شد",
        "rejected" => "رد شد",
        "cancelled" => "لغو‌شده",
        _ => "نامشخص",
    }
}

pub fn reason_label(s: &str) -> &'static str {
    match s {
        "defective" => "کالای معیوب/خراب",
        "wrong" => "ارسال کالای اشتباه",
        "changed_mind" => "انصراف از خرید",
        "not_as_described" => "مغایرت با توضیحات",
        _ => "سایر",
    }
}

pub const RETURN_STATUSES: [&str; 6] = [
    "requested", "approved", "received", "refunded", "rejected", "cancelled",
];
pub const RETURN_REASONS: [&str; 5] = [
    "defective", "wrong", "changed_mind", "not_as_described", "other",
];

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReturnItem {
    pub product_name: String,
    pub variant_title: Option<String>,
    pub qty: i32,
    pub unit_price: i64,
}

impl ReturnItem {
    pub fn line_total_fa(&self) -> String {
        domain::Rial(self.unit_price * self.qty as i64).format_toman_fa()
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct StatusLog {
    pub status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}
impl StatusLog {
    pub fn status_fa(&self) -> &'static str {
        status_label(&self.status)
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
}

/// قلم انتخاب‌شده برای مرجوعی.
pub struct NewReturnItem {
    pub order_item_id: Uuid,
    pub product_name: String,
    pub variant_title: Option<String>,
    pub qty: i32,
    pub unit_price: i64,
}

/// ثبت یک درخواست مرجوعی با اقلامش (اتمیک) + ثبت اولین وضعیت.
pub async fn create(
    db: &Db,
    order_id: Uuid,
    user_id: Option<Uuid>,
    reason: &str,
    description: Option<&str>,
    items: &[NewReturnItem],
) -> Result<Return, sqlx::Error> {
    let mut tx = db.begin().await?;
    let ret = sqlx::query_as::<_, Return>(
        "INSERT INTO returns (order_id, user_id, reason, description) VALUES ($1,$2,$3,$4) RETURNING *",
    )
    .bind(order_id)
    .bind(user_id)
    .bind(reason)
    .bind(description)
    .fetch_one(&mut *tx)
    .await?;

    for it in items {
        sqlx::query(
            "INSERT INTO return_items (return_id, order_item_id, product_name, variant_title, qty, unit_price)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(ret.id)
        .bind(it.order_item_id)
        .bind(&it.product_name)
        .bind(&it.variant_title)
        .bind(it.qty)
        .bind(it.unit_price)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("INSERT INTO return_status_log (return_id, status, note) VALUES ($1,'requested','درخواست ثبت شد')")
        .bind(ret.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ret)
}

pub async fn find(db: &Db, id: Uuid) -> Result<Option<Return>, sqlx::Error> {
    sqlx::query_as::<_, Return>("SELECT * FROM returns WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn find_by_number(db: &Db, number: i64) -> Result<Option<Return>, sqlx::Error> {
    sqlx::query_as::<_, Return>("SELECT * FROM returns WHERE return_number = $1")
        .bind(number)
        .fetch_optional(db)
        .await
}

pub async fn items(db: &Db, return_id: Uuid) -> Result<Vec<ReturnItem>, sqlx::Error> {
    sqlx::query_as::<_, ReturnItem>(
        "SELECT product_name, variant_title, qty, unit_price FROM return_items WHERE return_id = $1",
    )
    .bind(return_id)
    .fetch_all(db)
    .await
}

pub async fn history(db: &Db, return_id: Uuid) -> Result<Vec<StatusLog>, sqlx::Error> {
    sqlx::query_as::<_, StatusLog>(
        "SELECT status, note, created_at FROM return_status_log WHERE return_id = $1 ORDER BY created_at",
    )
    .bind(return_id)
    .fetch_all(db)
    .await
}

pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<Return>, sqlx::Error> {
    sqlx::query_as::<_, Return>(
        "SELECT * FROM returns WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// آیا برای این سفارش قبلاً مرجوعی باز/فعال ثبت شده؟ (جلوگیری از تکرار.)
pub async fn exists_open_for_order(db: &Db, order_id: Uuid) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM returns WHERE order_id = $1 AND status NOT IN ('rejected','cancelled')",
    )
    .bind(order_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

/// تغییر وضعیت + ثبت در تاریخچه (اتمیک).
pub async fn update_status(
    db: &Db,
    id: Uuid,
    status: &str,
    refund_amount: Option<i64>,
    note: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE returns SET status = $1, refund_amount = COALESCE($2, refund_amount), admin_note = COALESCE($3, admin_note) WHERE id = $4")
        .bind(status)
        .bind(refund_amount)
        .bind(note)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO return_status_log (return_id, status, note) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(status)
        .bind(note)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn cancel_by_user(db: &Db, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let changed = sqlx::query(
        "UPDATE returns SET status='cancelled' WHERE id=$1 AND user_id=$2 AND status='requested'",
    )
    .bind(id)
    .bind(user_id)
    .execute(db)
    .await?
    .rows_affected();
    if changed > 0 {
        let _ = sqlx::query("INSERT INTO return_status_log (return_id, status, note) VALUES ($1,'cancelled','لغو توسط مشتری')")
            .bind(id)
            .execute(db)
            .await;
    }
    Ok(changed > 0)
}

// ---------- ادمین ----------

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdminReturnRow {
    pub id: Uuid,
    pub return_number: i64,
    pub order_number: i64,
    pub status: String,
    pub reason: String,
    pub customer_mobile: Option<String>,
    pub created_at: DateTime<Utc>,
}
impl AdminReturnRow {
    pub fn status_fa(&self) -> &'static str {
        status_label(&self.status)
    }
    pub fn reason_fa(&self) -> &'static str {
        reason_label(&self.reason)
    }
    pub fn number_fa(&self) -> String {
        domain::fa_digits(&self.return_number.to_string())
    }
    pub fn order_number_fa(&self) -> String {
        domain::fa_digits(&self.order_number.to_string())
    }
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
}

pub async fn list_all(db: &Db, status: Option<&str>) -> Result<Vec<AdminReturnRow>, sqlx::Error> {
    let base = r#"
        SELECT r.id, r.return_number, o.order_number, r.status, r.reason,
               u.mobile AS customer_mobile, r.created_at
        FROM returns r
        JOIN orders o ON o.id = r.order_id
        LEFT JOIN users u ON u.id = r.user_id
    "#;
    match status {
        Some(s) => {
            sqlx::query_as::<_, AdminReturnRow>(&format!(
                "{base} WHERE r.status = $1 ORDER BY r.created_at DESC LIMIT 200"
            ))
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminReturnRow>(&format!("{base} ORDER BY r.created_at DESC LIMIT 200"))
                .fetch_all(db)
                .await
        }
    }
}

pub async fn count_open(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM returns WHERE status IN ('requested','approved','received')",
    )
    .fetch_one(db)
    .await
}

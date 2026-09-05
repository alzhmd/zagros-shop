//! Repository کیف پول — موجودی روی `users.wallet_balance` همیشه برابر مجموع
//! `wallet_transactions.delta`؛ هر دو در یک تراکنش به‌روز می‌شوند (مثل باشگاه).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WalletTx {
    pub id: Uuid,
    pub user_id: Uuid,
    pub delta: i64,
    pub kind: String,
    pub ref_order: Option<Uuid>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WalletTx {
    pub fn is_credit(&self) -> bool {
        self.delta >= 0
    }
    pub fn amount_fa(&self) -> String {
        domain::Rial(self.delta.abs()).format_toman_fa()
    }
    pub fn kind_fa(&self) -> &'static str {
        match self.kind.as_str() {
            "topup" => "شارژ کیف پول",
            "spend" => "پرداخت از کیف پول",
            "cancel_refund" => "بازگشت وجه (لغو سفارش)",
            "return_refund" => "بازگشت وجه (مرجوعی)",
            "giftcard" => "کارت هدیه",
            "admin" => "تنظیم توسط مدیر",
            _ => "سایر",
        }
    }
}

/// موجودی فعلی کیف پول (ریال).
pub async fn balance(db: &Db, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT wallet_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
}

/// ثبت یک تراکنش کیف پول و به‌روزرسانی اتمیک موجودی.
/// برای برداشت، `delta` منفی بده. در صورت کمبود موجودی هنگام برداشت، خطا برمی‌گرداند.
pub async fn add_transaction(
    db: &Db,
    user_id: Uuid,
    delta: i64,
    kind: &str,
    ref_order: Option<Uuid>,
    note: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    // برداشت بیش از موجودی مجاز نیست (قفل ردیف کاربر).
    if delta < 0 {
        let bal = sqlx::query_scalar::<_, i64>(
            "SELECT wallet_balance FROM users WHERE id = $1 FOR UPDATE",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if bal + delta < 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("موجودی کیف پول کافی نیست".into()));
        }
    }
    sqlx::query(
        "INSERT INTO wallet_transactions (user_id, delta, kind, ref_order, note)
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(delta)
    .bind(kind)
    .bind(ref_order)
    .bind(note)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE users SET wallet_balance = wallet_balance + $1 WHERE id = $2")
        .bind(delta)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// بازگشت وجه به کیف پول هنگام لغو/ناموفقی سفارش — idempotent با ایندکس یکتا.
/// مقدار برگشتی: مبلغ واقعاً بازگشتی (۰ اگر قبلاً بازگشته یا مبلغ صفر بود).
pub async fn refund_order(
    db: &Db,
    user_id: Uuid,
    order_id: Uuid,
    amount_rial: i64,
    note: &str,
) -> Result<i64, sqlx::Error> {
    if amount_rial <= 0 {
        return Ok(0);
    }
    let mut tx = db.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO wallet_transactions (user_id, delta, kind, ref_order, note)
         VALUES ($1,$2,'cancel_refund',$3,$4)
         ON CONFLICT (ref_order) WHERE kind = 'cancel_refund' DO NOTHING",
    )
    .bind(user_id)
    .bind(amount_rial)
    .bind(order_id)
    .bind(note)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if inserted == 0 {
        tx.rollback().await?;
        return Ok(0);
    }
    sqlx::query("UPDATE users SET wallet_balance = wallet_balance + $1 WHERE id = $2")
        .bind(amount_rial)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(amount_rial)
}

/// بازگردانی مبلغ خرج‌شده‌ی کیف پول برای یک سفارش (هنگام لغو سفارش پرداخت‌نشده).
/// مجموع تراکنش‌های `spend` آن سفارش را به کیف پول برمی‌گرداند (idempotent با حذف spendها).
pub async fn reverse_order_spend(db: &Db, order_id: Uuid) -> Result<i64, sqlx::Error> {
    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT user_id, COALESCE(sum(-delta),0)::bigint FROM wallet_transactions
         WHERE ref_order = $1 AND kind = 'spend' GROUP BY user_id FOR UPDATE",
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((user_id, spent)) = row.filter(|(_, s)| *s > 0) else {
        tx.rollback().await?;
        return Ok(0);
    };
    sqlx::query("DELETE FROM wallet_transactions WHERE ref_order = $1 AND kind = 'spend'")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO wallet_transactions (user_id, delta, kind, ref_order, note)
         VALUES ($1,$2,'cancel_refund',$3,'بازگردانی مبلغ کیف پول به‌دلیل لغو سفارش')",
    )
    .bind(user_id)
    .bind(spent)
    .bind(order_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE users SET wallet_balance = wallet_balance + $1 WHERE id = $2")
        .bind(spent)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(spent)
}

pub async fn history(db: &Db, user_id: Uuid, limit: i64) -> Result<Vec<WalletTx>, sqlx::Error> {
    sqlx::query_as::<_, WalletTx>(
        "SELECT * FROM wallet_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
}

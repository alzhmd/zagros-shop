//! Repository باشگاه مشتریان — دفتر امتیاز (فاز ۵).
//! اصل: `users.points` همیشه برابر مجموع `loyalty_transactions.delta` همان کاربر است؛
//! هر دو در یک تراکنش به‌روز می‌شوند.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// نرخ کسب امتیاز خرید: به ازای هر این مقدار ریال، ۱ امتیاز.
/// ۱۰۰٬۰۰۰ ریال = ۱۰٬۰۰۰ تومان → ۱ امتیاز.
pub const RIAL_PER_POINT: i64 = 100_000;

/// امتیاز پاداش اتصال حساب به تلگرام.
pub const TELEGRAM_LINK_BONUS: i64 = 50;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LoyaltyTx {
    pub id: Uuid,
    pub user_id: Uuid,
    pub delta: i64,
    pub reason: String,
    pub ref_order: Option<Uuid>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl LoyaltyTx {
    pub fn is_earn(&self) -> bool {
        self.delta >= 0
    }
    pub fn reason_fa(&self) -> &'static str {
        match self.reason.as_str() {
            "purchase" => "خرید",
            "telegram" => "اتصال تلگرام",
            "signup" => "خوش‌آمدگویی",
            "redeem" => "استفاده از امتیاز",
            "manual" => "تنظیم دستی",
            _ => "سایر",
        }
    }
    /// مقدار با علامت برای نمایش: «+۱۲۰» یا «-۵۰».
    pub fn delta_signed(&self) -> String {
        let sign = if self.delta >= 0 { "+" } else { "" };
        format!("{sign}{}", self.delta)
    }
}

/// تعداد امتیاز کسب‌شده از یک مبلغ خرید (ریالی)، با نرخ `rial_per_point`.
pub fn points_for_purchase(amount_rial: i64, rial_per_point: i64) -> i64 {
    let rate = if rial_per_point <= 0 {
        RIAL_PER_POINT
    } else {
        rial_per_point
    };
    if amount_rial <= 0 {
        0
    } else {
        amount_rial / rate
    }
}

/// ثبت یک تراکنش امتیاز و هم‌زمان به‌روزرسانی موجودی کاربر (اتمیک).
/// `delta` مثبت = کسب، منفی = خرج.
pub async fn add_transaction(
    db: &Db,
    user_id: Uuid,
    delta: i64,
    reason: &str,
    ref_order: Option<Uuid>,
    note: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO loyalty_transactions (user_id, delta, reason, ref_order, note)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(delta)
    .bind(reason)
    .bind(ref_order)
    .bind(note)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
        .bind(delta)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// اعطای امتیاز خرید برای یک سفارش، **idempotent**: ایندکس یکتا روی
/// (ref_order WHERE reason='purchase') از اعطای دوباره جلوگیری می‌کند.
/// در صورت تکراری بودن، خطای یکتایی را بی‌صدا نادیده می‌گیریم و موجودی را دست‌نخورده می‌گذاریم.
/// مقدار برگشتی: امتیاز واقعاً اعطاشده (۰ اگر قبلاً اعطا شده یا مبلغ ناچیز بود).
pub async fn award_purchase_points(
    db: &Db,
    user_id: Uuid,
    order_id: Uuid,
    amount_rial: i64,
    rial_per_point: i64,
) -> Result<i64, sqlx::Error> {
    let points = points_for_purchase(amount_rial, rial_per_point);
    if points == 0 {
        return Ok(0);
    }
    let mut tx = db.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO loyalty_transactions (user_id, delta, reason, ref_order)
         VALUES ($1, $2, 'purchase', $3)
         ON CONFLICT (ref_order) WHERE reason = 'purchase' DO NOTHING",
    )
    .bind(user_id)
    .bind(points)
    .bind(order_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if inserted == 0 {
        // قبلاً اعطا شده؛ بدون تغییر موجودی.
        tx.rollback().await?;
        return Ok(0);
    }
    sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
        .bind(points)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(points)
}

/// بازگرداندن امتیاز خرید یک سفارش (هنگام لغو). اگر امتیازی برای این سفارش
/// اعطا شده باشد، تراکنش معکوس ثبت و موجودی کاربر کم می‌شود (اتمیک، idempotent).
/// مقدار برگشتی: امتیاز بازگردانده‌شده (۰ اگر چیزی نبود).
pub async fn reverse_purchase_points(db: &Db, order_id: Uuid) -> Result<i64, sqlx::Error> {
    let mut tx = db.begin().await?;
    // تراکنش خرید این سفارش را پیدا کن (قفل).
    let row = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT user_id, delta FROM loyalty_transactions
         WHERE ref_order = $1 AND reason = 'purchase' FOR UPDATE",
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((user_id, delta)) = row else {
        tx.rollback().await?;
        return Ok(0);
    };

    // تراکنش خرید را حذف و یک تراکنش معکوس برای ردگیری ثبت کن.
    sqlx::query("DELETE FROM loyalty_transactions WHERE ref_order = $1 AND reason = 'purchase'")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO loyalty_transactions (user_id, delta, reason, note)
         VALUES ($1, $2, 'manual', 'بازگردانی امتیاز به‌دلیل لغو سفارش')",
    )
    .bind(user_id)
    .bind(-delta)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE users SET points = points - $1 WHERE id = $2")
        .bind(delta)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(delta)
}

pub async fn history(db: &Db, user_id: Uuid, limit: i64) -> Result<Vec<LoyaltyTx>, sqlx::Error> {
    sqlx::query_as::<_, LoyaltyTx>(
        "SELECT * FROM loyalty_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
}

/// موجودی فعلی امتیاز (از جدول users).
pub async fn balance(db: &Db, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT points FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
}

/// آیا کاربر تراکنشی با این دلیل دارد؟ (برای پاداش‌های یک‌باره مثل اتصال تلگرام).
pub async fn has_reason(db: &Db, user_id: Uuid, reason: &str) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM loyalty_transactions WHERE user_id = $1 AND reason = $2",
    )
    .bind(user_id)
    .bind(reason)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purchase_points_rate() {
        // ۵٬۰۰۰٬۰۰۰ ریال با نرخ پیش‌فرض ۱۰۰٬۰۰۰ → ۵۰ امتیاز
        assert_eq!(points_for_purchase(5_000_000, RIAL_PER_POINT), 50);
        assert_eq!(points_for_purchase(0, RIAL_PER_POINT), 0);
        assert_eq!(points_for_purchase(99_999, RIAL_PER_POINT), 0);
        assert_eq!(points_for_purchase(-100, RIAL_PER_POINT), 0);
        // نرخ سفارشی: هر ۵۰٬۰۰۰ ریال = ۱ امتیاز → دو برابر
        assert_eq!(points_for_purchase(5_000_000, 50_000), 100);
        // نرخ نامعتبر (۰) به پیش‌فرض برمی‌گردد
        assert_eq!(points_for_purchase(5_000_000, 0), 50);
    }
}

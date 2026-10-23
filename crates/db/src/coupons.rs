//! Repository کد تخفیف — اعتبارسنجی و محاسبه‌ی تخفیف (فاز ۵).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Coupon {
    pub id: Uuid,
    pub code: String,
    pub description: Option<String>,
    pub kind: String, // percent | fixed
    pub value: i64,
    pub min_order: i64,
    pub max_discount: Option<i64>,
    pub usage_limit: Option<i32>,
    pub used_count: i32,
    pub per_user_limit: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl Coupon {
    /// تخفیف ریالی این کوپن روی مبلغ `subtotal` (ریال). هرگز بیش از خود مبلغ.
    pub fn discount_for(&self, subtotal: i64) -> i64 {
        let raw = match self.kind.as_str() {
            "percent" => subtotal * self.value / 100,
            _ => self.value, // fixed
        };
        let capped = match self.max_discount {
            Some(cap) if self.kind == "percent" => raw.min(cap),
            _ => raw,
        };
        capped.min(subtotal).max(0)
    }

    pub fn is_percent(&self) -> bool {
        self.kind == "percent"
    }

    /// نمایش مقدار تخفیف: «۱۰٪» برای درصدی، یا مبلغ تومانی برای ثابت.
    pub fn value_display(&self) -> String {
        if self.is_percent() {
            format!("{}٪", domain::fa_digits(&self.value.to_string()))
        } else {
            domain::Rial(self.value).format_toman_fa()
        }
    }

    pub fn min_order_fa(&self) -> String {
        domain::Rial(self.min_order).format_toman_fa()
    }

    /// آیا کوپن منقضی شده؟
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| e < Utc::now()).unwrap_or(false)
    }
}

/// نرمال‌سازی کد کوپن: حذف فاصله، حروف بزرگ.
pub fn normalize_code(input: &str) -> String {
    input.trim().to_uppercase()
}

/// دلیل رد یک کوپن (برای پیام فارسی مناسب در UI).
#[derive(Debug, Clone, PartialEq)]
pub enum CouponReject {
    NotFound,
    Inactive,
    NotStarted,
    Expired,
    UsageExhausted,
    PerUserExhausted,
    BelowMinOrder(i64), // حداقل مبلغ لازم (ریال)
}

impl CouponReject {
    pub fn message_fa(&self) -> String {
        match self {
            CouponReject::NotFound => "کد تخفیف یافت نشد.".into(),
            CouponReject::Inactive => "این کد تخفیف غیرفعال است.".into(),
            CouponReject::NotStarted => "زمان استفاده از این کد هنوز نرسیده است.".into(),
            CouponReject::Expired => "این کد تخفیف منقضی شده است.".into(),
            CouponReject::UsageExhausted => "ظرفیت استفاده از این کد تکمیل شده است.".into(),
            CouponReject::PerUserExhausted => "شما قبلاً از این کد استفاده کرده‌اید.".into(),
            CouponReject::BelowMinOrder(min) => format!(
                "این کد برای سفارش‌های بالای {} معتبر است.",
                domain::Rial(*min).format_toman_fa()
            ),
        }
    }
}

pub async fn find_by_code(db: &Db, code: &str) -> Result<Option<Coupon>, sqlx::Error> {
    sqlx::query_as::<_, Coupon>("SELECT * FROM coupons WHERE code = $1")
        .bind(normalize_code(code))
        .fetch_optional(db)
        .await
}

/// تعداد دفعاتی که یک کاربر از این کوپن استفاده کرده (برای سقف per_user).
pub async fn user_redemption_count(
    db: &Db,
    coupon_id: Uuid,
    user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM coupon_redemptions WHERE coupon_id = $1 AND user_id = $2",
    )
    .bind(coupon_id)
    .bind(user_id)
    .fetch_one(db)
    .await
}

/// اعتبارسنجی کامل یک کوپن برای یک کاربر و مبلغ مشخص.
/// در صورت معتبر بودن، خود کوپن و مبلغ تخفیف را برمی‌گرداند.
pub async fn validate(
    db: &Db,
    code: &str,
    user_id: Option<Uuid>,
    subtotal: i64,
) -> Result<Result<(Coupon, i64), CouponReject>, sqlx::Error> {
    let Some(c) = find_by_code(db, code).await? else {
        return Ok(Err(CouponReject::NotFound));
    };
    if !c.is_active {
        return Ok(Err(CouponReject::Inactive));
    }
    let now = Utc::now();
    if let Some(start) = c.starts_at {
        if now < start {
            return Ok(Err(CouponReject::NotStarted));
        }
    }
    if let Some(exp) = c.expires_at {
        if now > exp {
            return Ok(Err(CouponReject::Expired));
        }
    }
    if let Some(limit) = c.usage_limit {
        if c.used_count >= limit {
            return Ok(Err(CouponReject::UsageExhausted));
        }
    }
    if subtotal < c.min_order {
        return Ok(Err(CouponReject::BelowMinOrder(c.min_order)));
    }
    if let Some(uid) = user_id {
        let used = user_redemption_count(db, c.id, uid).await?;
        if used >= c.per_user_limit as i64 {
            return Ok(Err(CouponReject::PerUserExhausted));
        }
    }
    let discount = c.discount_for(subtotal);
    Ok(Ok((c, discount)))
}

/// ثبت استفاده‌ی کوپن (افزایش used_count + رکورد redemption)، اتمیک.
pub async fn record_redemption(
    db: &Db,
    coupon_id: Uuid,
    user_id: Option<Uuid>,
    order_id: Uuid,
    amount: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    // افزایش شمارنده فقط تا سقف مجاز (جلوگیری از عبور از usage_limit در شرایط هم‌زمانی).
    let bumped = sqlx::query(
        "UPDATE coupons SET used_count = used_count + 1
         WHERE id = $1 AND (usage_limit IS NULL OR used_count < usage_limit)",
    )
    .bind(coupon_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if bumped == 0 {
        tracing::warn!(coupon = %coupon_id, "سقف استفاده‌ی کوپن پر بود هنگام ثبت — تخفیف اعمال شد ولی شمارنده افزایش نیافت");
    }
    sqlx::query(
        "INSERT INTO coupon_redemptions (coupon_id, user_id, order_id, amount)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(coupon_id)
    .bind(user_id)
    .bind(order_id)
    .bind(amount)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// فهرست همه‌ی کوپن‌ها (برای پنل ادمین).
pub async fn list_all(db: &Db) -> Result<Vec<Coupon>, sqlx::Error> {
    sqlx::query_as::<_, Coupon>("SELECT * FROM coupons ORDER BY created_at DESC")
        .fetch_all(db)
        .await
}

// ---------- CRUD ادمین ----------

pub async fn find_by_id(db: &Db, id: Uuid) -> Result<Option<Coupon>, sqlx::Error> {
    sqlx::query_as::<_, Coupon>("SELECT * FROM coupons WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn code_exists(
    db: &Db,
    code: &str,
    exclude_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM coupons WHERE code = $1 AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(normalize_code(code))
    .bind(exclude_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

/// داده‌های ساخت/ویرایش کوپن. مبالغ (value در نوع fixed، min_order، max_discount) ریالی‌اند.
pub struct CouponInput {
    pub code: String,
    pub description: Option<String>,
    pub kind: String, // percent | fixed
    pub value: i64,
    pub min_order: i64,
    pub max_discount: Option<i64>,
    pub usage_limit: Option<i32>,
    pub per_user_limit: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

pub async fn create(db: &Db, c: &CouponInput) -> Result<Coupon, sqlx::Error> {
    sqlx::query_as::<_, Coupon>(
        r#"INSERT INTO coupons
           (code, description, kind, value, min_order, max_discount, usage_limit, per_user_limit, expires_at, is_active)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *"#,
    )
    .bind(normalize_code(&c.code))
    .bind(&c.description)
    .bind(&c.kind)
    .bind(c.value)
    .bind(c.min_order)
    .bind(c.max_discount)
    .bind(c.usage_limit)
    .bind(c.per_user_limit)
    .bind(c.expires_at)
    .bind(c.is_active)
    .fetch_one(db)
    .await
}

pub async fn update(db: &Db, id: Uuid, c: &CouponInput) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE coupons SET
           code=$2, description=$3, kind=$4, value=$5, min_order=$6, max_discount=$7,
           usage_limit=$8, per_user_limit=$9, expires_at=$10, is_active=$11
           WHERE id=$1"#,
    )
    .bind(id)
    .bind(normalize_code(&c.code))
    .bind(&c.description)
    .bind(&c.kind)
    .bind(c.value)
    .bind(c.min_order)
    .bind(c.max_discount)
    .bind(c.usage_limit)
    .bind(c.per_user_limit)
    .bind(c.expires_at)
    .bind(c.is_active)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_active(db: &Db, id: Uuid, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE coupons SET is_active = $2 WHERE id = $1")
        .bind(id)
        .bind(active)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM coupons WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coupon(kind: &str, value: i64, max: Option<i64>) -> Coupon {
        Coupon {
            id: Uuid::nil(),
            code: "X".into(),
            description: None,
            kind: kind.into(),
            value,
            min_order: 0,
            max_discount: max,
            usage_limit: None,
            used_count: 0,
            per_user_limit: 1,
            starts_at: None,
            expires_at: None,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn percent_discount() {
        // ۱۰٪ از ۱٬۰۰۰٬۰۰۰ ریال = ۱۰۰٬۰۰۰
        assert_eq!(coupon("percent", 10, None).discount_for(1_000_000), 100_000);
    }

    #[test]
    fn percent_discount_capped() {
        // ۲۰٪ از ۱٬۰۰۰٬۰۰۰ = ۲۰۰٬۰۰۰ ولی سقف ۵۰٬۰۰۰
        assert_eq!(
            coupon("percent", 20, Some(50_000)).discount_for(1_000_000),
            50_000
        );
    }

    #[test]
    fn fixed_discount_never_exceeds_subtotal() {
        assert_eq!(
            coupon("fixed", 5_000_000, None).discount_for(3_000_000),
            3_000_000
        );
    }

    #[test]
    fn normalize() {
        assert_eq!(normalize_code("  welcome10 "), "WELCOME10");
    }
}

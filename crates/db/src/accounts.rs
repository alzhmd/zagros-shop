//! Repository کاربران، OTP و آدرس‌ها.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub mobile: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub password_hash: Option<String>,
    pub role: String,
    pub points: i64,
    pub wallet_balance: i64,
    pub birthdate: Option<chrono::NaiveDate>,
    pub gender: Option<String>,
    pub telegram_id: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// یافتن کاربر با موبایل.
pub async fn find_by_mobile(db: &Db, mobile: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE mobile = $1")
        .bind(mobile)
        .fetch_optional(db)
        .await
}

pub async fn find_by_id(db: &Db, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// ساخت کاربر جدید با موبایل (یا برگرداندن موجود).
pub async fn upsert_by_mobile(db: &Db, mobile: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (mobile) VALUES ($1)
         ON CONFLICT (mobile) DO UPDATE SET mobile = EXCLUDED.mobile
         RETURNING *",
    )
    .bind(mobile)
    .fetch_one(db)
    .await
}

/// بوت‌استرپ مدیر اول: کاربر با این موبایل را می‌سازد (در صورت نبود) و نقشش را admin می‌کند.
/// idempotent — هر بار راه‌اندازی بی‌خطر اجرا می‌شود.
pub async fn ensure_admin(db: &Db, mobile: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (mobile, role) VALUES ($1, 'admin')
         ON CONFLICT (mobile) DO UPDATE SET role = 'admin'",
    )
    .bind(mobile)
    .execute(db)
    .await?;
    Ok(())
}

/// ذخیره‌ی کد OTP هش‌شده.
pub async fn store_otp(
    db: &Db,
    mobile: &str,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO otp_codes (mobile, code_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(mobile)
        .bind(code_hash)
        .bind(expires_at)
        .execute(db)
        .await?;
    Ok(())
}

#[derive(Debug, FromRow)]
pub struct OtpRow {
    pub id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
    pub attempts: i32,
    pub consumed: bool,
}

/// آخرین OTP فعال برای یک موبایل.
pub async fn latest_otp(db: &Db, mobile: &str) -> Result<Option<OtpRow>, sqlx::Error> {
    sqlx::query_as::<_, OtpRow>(
        "SELECT id, code_hash, expires_at, attempts, consumed
         FROM otp_codes WHERE mobile = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(mobile)
    .fetch_optional(db)
    .await
}

pub async fn mark_otp_consumed(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE otp_codes SET consumed = true WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn increment_otp_attempts(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE otp_codes SET attempts = attempts + 1 WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// شمارش OTPهای اخیر (برای محدودیت نرخ).
pub async fn recent_otp_count(db: &Db, mobile: &str, minutes: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM otp_codes
         WHERE mobile = $1 AND created_at > now() - ($2 || ' minutes')::interval",
    )
    .bind(mobile)
    .bind(minutes.to_string())
    .fetch_one(db)
    .await
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub receiver: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub postal_code: Option<String>,
    pub line: String,
    pub is_default: bool,
}

pub async fn list_addresses(db: &Db, user_id: Uuid) -> Result<Vec<Address>, sqlx::Error> {
    sqlx::query_as::<_, Address>(
        "SELECT * FROM addresses WHERE user_id = $1 ORDER BY is_default DESC, created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn find_address(
    db: &Db,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<Address>, sqlx::Error> {
    sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(db)
        .await
}

pub async fn default_address(db: &Db, user_id: Uuid) -> Result<Option<Address>, sqlx::Error> {
    sqlx::query_as::<_, Address>(
        "SELECT * FROM addresses WHERE user_id = $1 ORDER BY is_default DESC, created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub struct NewAddress {
    pub receiver: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub postal_code: Option<String>,
    pub line: String,
    pub make_default: bool,
}

/// افزودن آدرس. اگر نخستین آدرس باشد یا make_default ست شده باشد، پیش‌فرض می‌شود.
pub async fn add_address(db: &Db, user_id: Uuid, a: &NewAddress) -> Result<Uuid, sqlx::Error> {
    let mut tx = db.begin().await?;
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM addresses WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    let is_default = a.make_default || count == 0;
    if is_default {
        sqlx::query("UPDATE addresses SET is_default = false WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO addresses (user_id, receiver, phone, province, city, postal_code, line, is_default)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(user_id)
    .bind(&a.receiver)
    .bind(&a.phone)
    .bind(&a.province)
    .bind(&a.city)
    .bind(&a.postal_code)
    .bind(&a.line)
    .bind(is_default)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn set_default_address(db: &Db, user_id: Uuid, id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE addresses SET is_default = false WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE addresses SET is_default = true WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_address(db: &Db, user_id: Uuid, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM addresses WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// به‌روزرسانی پروفایل کاربر (نام و ایمیل). ایمیل خالی → NULL.
pub async fn update_profile(
    db: &Db,
    user_id: Uuid,
    full_name: Option<&str>,
    email: Option<&str>,
    birthdate: Option<chrono::NaiveDate>,
    gender: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET full_name = $2, email = $3, birthdate = $4, gender = $5 WHERE id = $1",
    )
    .bind(user_id)
    .bind(full_name)
    .bind(email)
    .bind(birthdate)
    .bind(gender)
    .execute(db)
    .await?;
    Ok(())
}

/// حذف حساب (نرم): غیرفعال‌سازی + ناشناس‌سازی اطلاعات شخصی. تاریخچه‌ی سفارش حفظ می‌شود.
pub async fn soft_delete(db: &Db, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET is_active = false, full_name = NULL, email = NULL,
                birthdate = NULL, gender = NULL, telegram_id = NULL,
                mobile = 'deleted-' || id::text
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

// ---------- اتصال حساب به تلگرام (فاز ۵) ----------

/// یافتن کاربر با شناسه‌ی تلگرام (برای ربات).
pub async fn find_by_telegram_id(db: &Db, telegram_id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_optional(db)
        .await
}

/// ساخت یک کد اتصال یکبارمصرف برای کاربر (با انقضا). کدهای مصرف‌نشده‌ی قبلی پاک می‌شوند.
pub async fn create_link_code(
    db: &Db,
    user_id: Uuid,
    code: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM telegram_link_codes WHERE user_id = $1 AND consumed = false")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO telegram_link_codes (code, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(code)
        .bind(user_id)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// نتیجه‌ی تلاش برای اتصال حساب از طریق کد.
pub enum LinkOutcome {
    Linked(User),
    AlreadyLinkedToOther,
    InvalidOrExpired,
}

/// مصرف کد اتصال و وصل‌کردن `telegram_id` به حساب کاربر (اتمیک).
pub async fn consume_link_code(
    db: &Db,
    code: &str,
    telegram_id: i64,
) -> Result<LinkOutcome, sqlx::Error> {
    let mut tx = db.begin().await?;

    // کد معتبر و مصرف‌نشده و منقضی‌نشده را قفل می‌کنیم.
    let row = sqlx::query_as::<_, (Uuid,)>(
        "SELECT user_id FROM telegram_link_codes
         WHERE code = $1 AND consumed = false AND expires_at > now()
         FOR UPDATE",
    )
    .bind(code)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((user_id,)) = row else {
        tx.rollback().await?;
        return Ok(LinkOutcome::InvalidOrExpired);
    };

    // اگر این telegram_id قبلاً به کاربر دیگری وصل است، اجازه نده.
    let owner = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(existing) = owner {
        if existing != user_id {
            tx.rollback().await?;
            return Ok(LinkOutcome::AlreadyLinkedToOther);
        }
    }

    sqlx::query("UPDATE telegram_link_codes SET consumed = true WHERE code = $1")
        .bind(code)
        .execute(&mut *tx)
        .await?;
    let user =
        sqlx::query_as::<_, User>("UPDATE users SET telegram_id = $1 WHERE id = $2 RETURNING *")
            .bind(telegram_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;
    Ok(LinkOutcome::Linked(user))
}

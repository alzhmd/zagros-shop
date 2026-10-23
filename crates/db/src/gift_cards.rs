//! Repository کارت هدیه — صدور و مصرف (واریز به کیف پول).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct GiftCard {
    pub id: Uuid,
    pub code: String,
    pub initial_rial: i64,
    pub balance_rial: i64,
    pub issued_to: Option<Uuid>,
    pub redeemed_by: Option<Uuid>,
    pub redeemed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl GiftCard {
    pub fn balance_fa(&self) -> String {
        domain::Rial(self.balance_rial).format_toman_fa()
    }
    pub fn initial_fa(&self) -> String {
        domain::Rial(self.initial_rial).format_toman_fa()
    }
}

/// صدور یک کارت هدیه با کد مشخص.
pub async fn issue(
    db: &Db,
    code: &str,
    amount_rial: i64,
    issued_to: Option<Uuid>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO gift_cards (code, initial_rial, balance_rial, issued_to, expires_at)
         VALUES ($1,$2,$2,$3,$4)",
    )
    .bind(code)
    .bind(amount_rial)
    .bind(issued_to)
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok(())
}

/// نتیجه‌ی تلاش برای استفاده از کارت هدیه.
pub enum RedeemError {
    NotFound,
    Expired,
    AlreadyUsed,
}

impl RedeemError {
    pub fn message_fa(&self) -> &'static str {
        match self {
            RedeemError::NotFound => "کد کارت هدیه نامعتبر است.",
            RedeemError::Expired => "این کارت هدیه منقضی شده است.",
            RedeemError::AlreadyUsed => "این کارت هدیه قبلاً استفاده شده است.",
        }
    }
}

/// مصرف کارت هدیه: موجودی را صفر و مبلغ را برمی‌گرداند تا فراخوان به کیف پول واریز کند.
/// اتمیک با قفل ردیف. مقدار موفق = مبلغ ریالی واریزشدنی.
pub async fn redeem(
    db: &Db,
    code: &str,
    user_id: Uuid,
) -> Result<Result<i64, RedeemError>, sqlx::Error> {
    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, GiftCard>("SELECT * FROM gift_cards WHERE code = $1 FOR UPDATE")
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(card) = row else {
        tx.rollback().await?;
        return Ok(Err(RedeemError::NotFound));
    };
    if card.redeemed_at.is_some() || card.balance_rial <= 0 {
        tx.rollback().await?;
        return Ok(Err(RedeemError::AlreadyUsed));
    }
    if let Some(exp) = card.expires_at {
        if exp < Utc::now() {
            tx.rollback().await?;
            return Ok(Err(RedeemError::Expired));
        }
    }
    let amount = card.balance_rial;
    sqlx::query(
        "UPDATE gift_cards SET balance_rial = 0, redeemed_by = $2, redeemed_at = now() WHERE id = $1",
    )
    .bind(card.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(amount))
}

pub async fn list_all(db: &Db, limit: i64) -> Result<Vec<GiftCard>, sqlx::Error> {
    sqlx::query_as::<_, GiftCard>("SELECT * FROM gift_cards ORDER BY created_at DESC LIMIT $1")
        .bind(limit)
        .fetch_all(db)
        .await
}

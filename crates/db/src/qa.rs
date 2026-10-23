//! Repository پرسش و پاسخ محصول.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Question {
    pub id: Uuid,
    pub product_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub body: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl Question {
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Answer {
    pub id: Uuid,
    pub question_id: Uuid,
    pub body: String,
    pub is_staff: bool,
    pub created_at: DateTime<Utc>,
}

/// پرسش جدید (در انتظار تأیید).
pub async fn ask(
    db: &Db,
    product_id: Uuid,
    user_id: Option<Uuid>,
    author_name: &str,
    body: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO product_questions (product_id, user_id, author_name, body)
         VALUES ($1,$2,$3,$4)",
    )
    .bind(product_id)
    .bind(user_id)
    .bind(author_name)
    .bind(body)
    .execute(db)
    .await?;
    Ok(())
}

/// پرسش‌های تأییدشده‌ی یک محصول، هرکدام با پاسخ‌هایشان.
pub async fn approved_for_product(
    db: &Db,
    product_id: Uuid,
) -> Result<Vec<(Question, Vec<Answer>)>, sqlx::Error> {
    let questions = sqlx::query_as::<_, Question>(
        "SELECT * FROM product_questions WHERE product_id = $1 AND status = 'approved'
         ORDER BY created_at DESC LIMIT 50",
    )
    .bind(product_id)
    .fetch_all(db)
    .await?;

    let mut out = Vec::with_capacity(questions.len());
    for q in questions {
        let answers = sqlx::query_as::<_, Answer>(
            "SELECT * FROM product_answers WHERE question_id = $1 ORDER BY created_at",
        )
        .bind(q.id)
        .fetch_all(db)
        .await?;
        out.push((q, answers));
    }
    Ok(out)
}

// ---------- پرسش‌های کاربر ----------

/// یک ردیف پرسشِ کاربر برای صفحه‌ی «پرسش‌های من» (با محصول و اولین پاسخ).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MyQuestionRow {
    pub id: Uuid,
    pub product_name: String,
    pub product_slug: String,
    pub body: String,
    pub status: String,
    pub answer: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl MyQuestionRow {
    pub fn date_fa(&self) -> String {
        domain::fa_digits(&self.created_at.format("%Y/%m/%d").to_string())
    }
    pub fn status_fa(&self) -> &'static str {
        match self.status.as_str() {
            "approved" => "پاسخ داده‌شده",
            "rejected" => "ردشده",
            _ => "در انتظار پاسخ",
        }
    }
    pub fn status_key(&self) -> &str {
        match self.status.as_str() {
            "approved" | "rejected" => &self.status,
            _ => "pending",
        }
    }
}

/// همه‌ی پرسش‌های یک کاربر (هر وضعیتی) — جدیدترین اول.
pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<MyQuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, MyQuestionRow>(
        r#"
        SELECT q.id, p.name AS product_name, p.slug AS product_slug, q.body, q.status,
               (SELECT a.body FROM product_answers a WHERE a.question_id = q.id
                ORDER BY a.created_at LIMIT 1) AS answer,
               q.created_at
        FROM product_questions q
        JOIN products p ON p.id = q.product_id
        WHERE q.user_id = $1
        ORDER BY q.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

// ---------- مدیریت (ادمین) ----------

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdminQuestionRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_slug: String,
    pub author_name: String,
    pub body: String,
    pub status: String,
    pub answer: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn list_all(
    db: &Db,
    status_filter: Option<&str>,
) -> Result<Vec<AdminQuestionRow>, sqlx::Error> {
    let base = r#"
        SELECT q.id, q.product_id, p.name AS product_name, p.slug AS product_slug,
               q.author_name, q.body, q.status,
               (SELECT a.body FROM product_answers a WHERE a.question_id = q.id ORDER BY a.created_at LIMIT 1) AS answer,
               q.created_at
        FROM product_questions q JOIN products p ON p.id = q.product_id
    "#;
    match status_filter {
        Some(s) => {
            sqlx::query_as::<_, AdminQuestionRow>(&format!(
                "{base} WHERE q.status = $1 ORDER BY q.created_at DESC LIMIT 300"
            ))
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminQuestionRow>(&format!(
                "{base} ORDER BY q.created_at DESC LIMIT 300"
            ))
            .fetch_all(db)
            .await
        }
    }
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE product_questions SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// ثبت پاسخ ادمین و تأیید خودکار پرسش.
pub async fn answer(db: &Db, question_id: Uuid, body: &str) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO product_answers (question_id, body, is_staff) VALUES ($1,$2,true)")
        .bind(question_id)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE product_questions SET status = 'approved' WHERE id = $1")
        .bind(question_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM product_questions WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// شمار پرسش‌های در انتظار (برای داشبورد ادمین).
pub async fn count_pending(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM product_questions WHERE status = 'pending'")
        .fetch_one(db)
        .await
}

/// اطلاعاتِ لازم برای اعلانِ پاسخ: (user_id, نام محصول, نامک).
pub async fn notify_info(
    db: &Db,
    question_id: Uuid,
) -> Result<Option<(Option<Uuid>, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (Option<Uuid>, String, String)>(
        "SELECT q.user_id, p.name, p.slug FROM product_questions q
         JOIN products p ON p.id = q.product_id WHERE q.id = $1",
    )
    .bind(question_id)
    .fetch_optional(db)
    .await
}

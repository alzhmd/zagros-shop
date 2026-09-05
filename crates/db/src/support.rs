//! Repository پشتیبانی — تیکت و چت زنده (فاز ۴).

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

// ===================== تیکت =====================

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Ticket {
    pub id: Uuid,
    pub ticket_number: i64,
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub email: Option<String>,
    pub subject: String,
    pub department: String,
    pub priority: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ticket {
    pub fn status_fa(&self) -> &'static str {
        ticket_status_label(&self.status)
    }
    pub fn department_fa(&self) -> &'static str {
        department_label(&self.department)
    }
    pub fn priority_fa(&self) -> &'static str {
        priority_label(&self.priority)
    }
    pub fn is_closed(&self) -> bool {
        self.status == "closed"
    }
}

pub fn ticket_status_label(s: &str) -> &'static str {
    match s {
        "open" => "باز",
        "answered" => "پاسخ داده‌شده",
        "closed" => "بسته‌شده",
        _ => "نامشخص",
    }
}

pub fn department_label(s: &str) -> &'static str {
    match s {
        "sales" => "فروش",
        "technical" => "فنی",
        "billing" => "مالی",
        _ => "عمومی",
    }
}

pub fn priority_label(s: &str) -> &'static str {
    match s {
        "low" => "کم",
        "high" => "بالا",
        _ => "عادی",
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TicketMessage {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub sender: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl TicketMessage {
    pub fn is_staff(&self) -> bool {
        self.sender == "staff"
    }
    pub fn sender_fa(&self) -> &'static str {
        if self.is_staff() {
            "پشتیبانی"
        } else {
            "مشتری"
        }
    }
}

pub struct NewTicket {
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub email: Option<String>,
    pub subject: String,
    pub department: String,
    pub priority: String,
    pub body: String,
}

/// ساخت تیکت به‌همراه نخستین پیام مشتری، اتمیک.
pub async fn create_ticket(db: &Db, t: &NewTicket) -> Result<Ticket, sqlx::Error> {
    let mut tx = db.begin().await?;
    let ticket = sqlx::query_as::<_, Ticket>(
        r#"INSERT INTO tickets (user_id, full_name, email, subject, department, priority)
           VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
    )
    .bind(t.user_id)
    .bind(&t.full_name)
    .bind(&t.email)
    .bind(&t.subject)
    .bind(&t.department)
    .bind(&t.priority)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO ticket_messages (ticket_id, sender, body) VALUES ($1, 'customer', $2)")
        .bind(ticket.id)
        .bind(&t.body)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ticket)
}

pub async fn find_ticket(db: &Db, id: Uuid) -> Result<Option<Ticket>, sqlx::Error> {
    sqlx::query_as::<_, Ticket>("SELECT * FROM tickets WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// یافتن با شماره‌ی تیکت (برای پیگیری مهمان).
pub async fn find_ticket_by_number(db: &Db, number: i64) -> Result<Option<Ticket>, sqlx::Error> {
    sqlx::query_as::<_, Ticket>("SELECT * FROM tickets WHERE ticket_number = $1")
        .bind(number)
        .fetch_optional(db)
        .await
}

pub async fn list_tickets_for_user(db: &Db, user_id: Uuid) -> Result<Vec<Ticket>, sqlx::Error> {
    sqlx::query_as::<_, Ticket>(
        "SELECT * FROM tickets WHERE user_id = $1 ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn list_all_tickets(
    db: &Db,
    status_filter: Option<&str>,
) -> Result<Vec<Ticket>, sqlx::Error> {
    match status_filter {
        Some(s) => {
            sqlx::query_as::<_, Ticket>(
                "SELECT * FROM tickets WHERE status = $1 ORDER BY updated_at DESC",
            )
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, Ticket>("SELECT * FROM tickets ORDER BY updated_at DESC")
                .fetch_all(db)
                .await
        }
    }
}

pub async fn ticket_messages(db: &Db, ticket_id: Uuid) -> Result<Vec<TicketMessage>, sqlx::Error> {
    sqlx::query_as::<_, TicketMessage>(
        "SELECT * FROM ticket_messages WHERE ticket_id = $1 ORDER BY created_at",
    )
    .bind(ticket_id)
    .fetch_all(db)
    .await
}

/// افزودن پیام به تیکت + تنظیم وضعیت جدید (اتمیک). تریگر `updated_at` را تازه می‌کند.
pub async fn add_ticket_message(
    db: &Db,
    ticket_id: Uuid,
    sender: &str,
    body: &str,
    new_status: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO ticket_messages (ticket_id, sender, body) VALUES ($1, $2, $3)")
        .bind(ticket_id)
        .bind(sender)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE tickets SET status = $1 WHERE id = $2")
        .bind(new_status)
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn set_ticket_status(db: &Db, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tickets SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// شمارش تیکت‌های باز (برای داشبورد ادمین).
pub async fn count_open_tickets(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tickets WHERE status != 'closed'")
        .fetch_one(db)
        .await
}

// ===================== چت زنده =====================

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub guest_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl ChatSession {
    /// برچسب نمایشی برای پنل اپراتور.
    pub fn display_name(&self) -> String {
        self.guest_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "مهمان".to_string())
    }
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sender: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    pub fn is_staff(&self) -> bool {
        self.sender == "staff"
    }
}

pub async fn create_chat_session(
    db: &Db,
    user_id: Option<Uuid>,
    guest_name: Option<&str>,
) -> Result<ChatSession, sqlx::Error> {
    sqlx::query_as::<_, ChatSession>(
        "INSERT INTO chat_sessions (user_id, guest_name) VALUES ($1, $2) RETURNING *",
    )
    .bind(user_id)
    .bind(guest_name)
    .fetch_one(db)
    .await
}

pub async fn find_chat_session(db: &Db, id: Uuid) -> Result<Option<ChatSession>, sqlx::Error> {
    sqlx::query_as::<_, ChatSession>("SELECT * FROM chat_sessions WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn list_active_chat_sessions(db: &Db) -> Result<Vec<ChatSession>, sqlx::Error> {
    sqlx::query_as::<_, ChatSession>(
        "SELECT * FROM chat_sessions WHERE status = 'active' ORDER BY created_at DESC",
    )
    .fetch_all(db)
    .await
}

/// ردیف جلسه‌ی چت برای پنل ادمین — با شمار پیام.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatSessionRow {
    pub id: Uuid,
    pub guest_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub message_count: i64,
}

impl ChatSessionRow {
    pub fn display_name(&self) -> String {
        self.guest_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "مهمان".to_string())
    }
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

/// فهرست جلسه‌های چت با فیلتر وضعیت اختیاری (active | closed | همه).
pub async fn list_chat_sessions(
    db: &Db,
    status: Option<&str>,
) -> Result<Vec<ChatSessionRow>, sqlx::Error> {
    let base = r#"
        SELECT s.id, s.guest_name, s.status, s.created_at,
               (SELECT count(*) FROM chat_messages m WHERE m.session_id = s.id) AS message_count
        FROM chat_sessions s
    "#;
    match status {
        Some(st) => {
            sqlx::query_as::<_, ChatSessionRow>(&format!(
                "{base} WHERE s.status = $1 ORDER BY s.created_at DESC LIMIT 200"
            ))
            .bind(st)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, ChatSessionRow>(&format!(
                "{base} ORDER BY s.created_at DESC LIMIT 200"
            ))
            .fetch_all(db)
            .await
        }
    }
}

/// ثبت یک پیام چت و برگرداندن ردیف کامل (برای پخش به مشترکین).
pub async fn add_chat_message(
    db: &Db,
    session_id: Uuid,
    sender: &str,
    body: &str,
) -> Result<ChatMessage, sqlx::Error> {
    sqlx::query_as::<_, ChatMessage>(
        "INSERT INTO chat_messages (session_id, sender, body) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(session_id)
    .bind(sender)
    .bind(body)
    .fetch_one(db)
    .await
}

pub async fn chat_messages(db: &Db, session_id: Uuid) -> Result<Vec<ChatMessage>, sqlx::Error> {
    sqlx::query_as::<_, ChatMessage>(
        "SELECT * FROM chat_messages WHERE session_id = $1 ORDER BY created_at",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
}

pub async fn close_chat_session(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_sessions SET status = 'closed' WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

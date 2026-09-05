//! Repository سیستم تعمیرات.

use crate::Db;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct RepairRequest {
    pub id: Uuid,
    pub ticket_number: i64,
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub phone: String,
    pub device_type: String,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub problem: String,
    pub method: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub status: String,
    pub technician_id: Option<Uuid>,
    pub cost_estimate: Option<i64>,
    pub final_cost: Option<i64>,
    pub admin_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RepairRequest {
    pub fn status_fa(&self) -> &'static str {
        status_label(&self.status)
    }
    pub fn method_fa(&self) -> &'static str {
        match self.method.as_str() {
            "onsite" => "اعزام کارشناس به محل",
            "dropoff" => "تحویل به انبار",
            _ => "—",
        }
    }
}

pub fn status_label(s: &str) -> &'static str {
    match s {
        "submitted" => "ثبت‌شده",
        "reviewing" => "در حال بررسی",
        "scheduled" => "زمان‌بندی‌شده",
        "in_progress" => "در حال تعمیر",
        "awaiting_parts" => "در انتظار قطعه",
        "ready" => "آماده‌ی تحویل",
        "completed" => "تکمیل‌شده",
        "cancelled" => "لغو‌شده",
        _ => "نامشخص",
    }
}

pub struct NewRepair {
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub phone: String,
    pub device_type: String,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub problem: String,
    pub method: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub address: Option<String>,
}

pub async fn create(db: &Db, r: &NewRepair) -> Result<RepairRequest, sqlx::Error> {
    let mut tx = db.begin().await?;
    let req = sqlx::query_as::<_, RepairRequest>(
        r#"INSERT INTO repair_requests
           (user_id, full_name, phone, device_type, brand, model, problem, method, province, city, address)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *"#,
    )
    .bind(r.user_id)
    .bind(&r.full_name)
    .bind(&r.phone)
    .bind(&r.device_type)
    .bind(&r.brand)
    .bind(&r.model)
    .bind(&r.problem)
    .bind(&r.method)
    .bind(&r.province)
    .bind(&r.city)
    .bind(&r.address)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO repair_status_log (repair_id, status, note) VALUES ($1, 'submitted', 'درخواست ثبت شد')")
        .bind(req.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(req)
}

pub async fn find(db: &Db, id: Uuid) -> Result<Option<RepairRequest>, sqlx::Error> {
    sqlx::query_as::<_, RepairRequest>("SELECT * FROM repair_requests WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// یافتن با شماره‌ی پیگیری (برای مهمان).
pub async fn find_by_ticket(db: &Db, ticket: i64) -> Result<Option<RepairRequest>, sqlx::Error> {
    sqlx::query_as::<_, RepairRequest>("SELECT * FROM repair_requests WHERE ticket_number = $1")
        .bind(ticket)
        .fetch_optional(db)
        .await
}

pub async fn list_for_user(db: &Db, user_id: Uuid) -> Result<Vec<RepairRequest>, sqlx::Error> {
    sqlx::query_as::<_, RepairRequest>(
        "SELECT * FROM repair_requests WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn list_all(db: &Db, status_filter: Option<&str>) -> Result<Vec<RepairRequest>, sqlx::Error> {
    match status_filter {
        Some(s) => {
            sqlx::query_as::<_, RepairRequest>(
                "SELECT * FROM repair_requests WHERE status = $1 ORDER BY created_at DESC",
            )
            .bind(s)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, RepairRequest>(
                "SELECT * FROM repair_requests ORDER BY created_at DESC",
            )
            .fetch_all(db)
            .await
        }
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
}

pub async fn status_history(db: &Db, repair_id: Uuid) -> Result<Vec<StatusLog>, sqlx::Error> {
    sqlx::query_as::<_, StatusLog>(
        "SELECT status, note, created_at FROM repair_status_log
         WHERE repair_id = $1 ORDER BY created_at",
    )
    .bind(repair_id)
    .fetch_all(db)
    .await
}

/// تغییر وضعیت + ثبت در تاریخچه (اتمیک).
pub async fn update_status(
    db: &Db,
    id: Uuid,
    status: &str,
    note: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE repair_requests SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO repair_status_log (repair_id, status, note) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(status)
        .bind(note)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// به‌روزرسانی جزئیات فنی تعمیر (برآورد هزینه، هزینه‌ی نهایی، تکنسین، یادداشت ادمین).
pub async fn set_details(
    db: &Db,
    id: Uuid,
    cost_estimate: Option<i64>,
    final_cost: Option<i64>,
    technician_id: Option<Uuid>,
    admin_note: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repair_requests
         SET cost_estimate = $2, final_cost = $3, technician_id = $4, admin_note = $5
         WHERE id = $1",
    )
    .bind(id)
    .bind(cost_estimate)
    .bind(final_cost)
    .bind(technician_id)
    .bind(admin_note)
    .execute(db)
    .await?;
    Ok(())
}

/// فهرست تکنسین‌ها/مدیران برای انتساب تعمیر.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct TechOption {
    pub id: Uuid,
    pub label: String,
}

pub async fn list_technicians(db: &Db) -> Result<Vec<TechOption>, sqlx::Error> {
    sqlx::query_as::<_, TechOption>(
        "SELECT id, COALESCE(NULLIF(full_name,''), mobile) AS label
         FROM users WHERE role IN ('technician','admin','operator') ORDER BY label",
    )
    .fetch_all(db)
    .await
}

/// شمارش بر اساس وضعیت (برای داشبورد ادمین).
pub async fn count_by_status(db: &Db) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, count(*)::bigint FROM repair_requests GROUP BY status",
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

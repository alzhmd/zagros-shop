use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use db::repairs::RepairRequest;
use db::support::{self, Ticket, TicketMessage};
use serde::Deserialize;
use uuid::Uuid;

/// یک گزینه‌ی فیلتر وضعیت با برچسب فارسی.
pub struct FilterOption {
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
}

const REPAIR_STATUSES: [&str; 8] = [
    "submitted",
    "reviewing",
    "scheduled",
    "in_progress",
    "awaiting_parts",
    "ready",
    "completed",
    "cancelled",
];
const TICKET_STATUSES: [&str; 3] = ["open", "answered", "closed"];

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct DashTemplate {
    site_name: String,
    repair_counts: Vec<(String, String, i64)>,
    total_repairs: i64,
    total_products: i64,
    active_products: i64,
    total_categories: i64,
    total_brands: i64,
    total_orders: i64,
    total_users: i64,
    open_tickets: i64,
    active_chats: i64,
    pending_reviews: i64,
    pending_questions: i64,
    open_returns: i64,
}

pub async fn dashboard(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let counts = db::repairs::count_by_status(&s.db).await?;
    let total_repairs: i64 = counts.iter().map(|(_, c)| c).sum();
    let repair_counts: Vec<(String, String, i64)> = counts
        .into_iter()
        .map(|(st, c)| (st.clone(), db::repairs::status_label(&st).to_string(), c))
        .collect();
    // همه‌ی شمارش‌ها در یک کوئری (به‌جای ۹ کوئری جداگانه).
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64)>(
        "SELECT
            (SELECT count(*) FROM products),
            (SELECT count(*) FROM products WHERE is_active),
            (SELECT count(*) FROM categories),
            (SELECT count(*) FROM brands),
            (SELECT count(*) FROM orders),
            (SELECT count(*) FROM users),
            (SELECT count(*) FROM tickets WHERE status <> 'closed'),
            (SELECT count(*) FROM chat_sessions WHERE status = 'active'),
            (SELECT count(*) FROM product_reviews WHERE status = 'pending'),
            (SELECT count(*) FROM product_questions WHERE status = 'pending'),
            (SELECT count(*) FROM returns WHERE status IN ('requested','approved','received'))",
    )
    .fetch_one(&s.db)
    .await?;
    let (
        total_products,
        active_products,
        total_categories,
        total_brands,
        total_orders,
        total_users,
        open_tickets,
        active_chats,
        pending_reviews,
        pending_questions,
        open_returns,
    ) = row;
    render(DashTemplate {
        site_name: s.site_name(),
        repair_counts,
        total_repairs,
        total_products,
        active_products,
        total_categories,
        total_brands,
        total_orders,
        total_users,
        open_tickets,
        active_chats,
        pending_reviews,
        pending_questions,
        open_returns,
    })
}

#[derive(Template)]
#[template(path = "admin/repairs.html")]
struct RepairsTemplate {
    site_name: String,
    repairs: Vec<RepairRequest>,
    statuses: Vec<FilterOption>,
    active_filter: String,
}

pub async fn repairs(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<StatusQuery>,
) -> AppResult<Html<String>> {
    let filter = q.status.filter(|st| REPAIR_STATUSES.contains(&st.as_str()));
    let repairs = db::repairs::list_all(&s.db, filter.as_deref()).await?;
    let statuses = REPAIR_STATUSES
        .iter()
        .map(|k| FilterOption {
            key: k,
            label: db::repairs::status_label(k),
        })
        .collect();
    render(RepairsTemplate {
        site_name: s.site_name(),
        repairs,
        statuses,
        active_filter: filter.unwrap_or_default(),
    })
}

#[derive(Template)]
#[template(path = "admin/repair_detail.html")]
struct RepairDetailTemplate {
    site_name: String,
    repair: RepairRequest,
    ticket_fa: String,
    history: Vec<db::repairs::StatusLog>,
    statuses: Vec<FilterOption>,
    technicians: Vec<db::repairs::TechOption>,
    cost_estimate_toman: String,
    final_cost_toman: String,
    technician_id: String,
}

pub async fn repair_detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let repair = db::repairs::find(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("درخواست".into()))?;
    let history = db::repairs::status_history(&s.db, id).await?;
    let ticket_fa = crate::filters::to_fa_digits(&repair.ticket_number.to_string());
    let technicians = db::repairs::list_technicians(&s.db)
        .await
        .unwrap_or_default();
    let statuses = REPAIR_STATUSES
        .iter()
        .map(|k| FilterOption {
            key: k,
            label: db::repairs::status_label(k),
        })
        .collect();
    let cost_estimate_toman = repair
        .cost_estimate
        .map(|c| (c / 10).to_string())
        .unwrap_or_default();
    let final_cost_toman = repair
        .final_cost
        .map(|c| (c / 10).to_string())
        .unwrap_or_default();
    let technician_id = repair
        .technician_id
        .map(|t| t.to_string())
        .unwrap_or_default();
    render(RepairDetailTemplate {
        site_name: s.site_name(),
        repair,
        ticket_fa,
        history,
        statuses,
        technicians,
        cost_estimate_toman,
        final_cost_toman,
        technician_id,
    })
}

#[derive(Deserialize)]
pub struct UpdateRepairForm {
    pub status: String,
    pub note: Option<String>,
}

pub async fn update_repair(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<UpdateRepairForm>,
) -> AppResult<axum::response::Response> {
    db::repairs::update_status(&s.db, id, &f.status, f.note.as_deref()).await?;
    Ok(Redirect::to(&format!("/admin/repairs/{}", id)).into_response())
}

#[derive(Deserialize)]
pub struct RepairDetailsForm {
    pub cost_estimate_toman: Option<String>,
    pub final_cost_toman: Option<String>,
    pub technician_id: Option<String>,
    pub admin_note: Option<String>,
}

pub async fn update_repair_details(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<RepairDetailsForm>,
) -> AppResult<axum::response::Response> {
    let to_rial = |v: &Option<String>| -> Option<i64> {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|s| crate::routes::admin_util::parse_digits(s).ok())
            .map(|t| t * 10)
    };
    let cost_estimate = to_rial(&f.cost_estimate_toman);
    let final_cost = to_rial(&f.final_cost_toman);
    let technician_id = crate::routes::admin_util::parse_uuid_opt(&f.technician_id);
    let admin_note = f
        .admin_note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    db::repairs::set_details(
        &s.db,
        id,
        cost_estimate,
        final_cost,
        technician_id,
        admin_note,
    )
    .await?;
    Ok(Redirect::to(&format!("/admin/repairs/{}", id)).into_response())
}

// ===================== پشتیبانی: تیکت‌ها =====================

#[derive(Template)]
#[template(path = "admin/tickets.html")]
struct AdminTicketsTemplate {
    site_name: String,
    tickets: Vec<Ticket>,
    statuses: Vec<FilterOption>,
    active_filter: String,
}

pub async fn tickets(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<StatusQuery>,
) -> AppResult<Html<String>> {
    let filter = q.status.filter(|st| TICKET_STATUSES.contains(&st.as_str()));
    let tickets = support::list_all_tickets(&s.db, filter.as_deref()).await?;
    let statuses = TICKET_STATUSES
        .iter()
        .map(|k| FilterOption {
            key: k,
            label: db::support::ticket_status_label(k),
        })
        .collect();
    render(AdminTicketsTemplate {
        site_name: s.site_name(),
        tickets,
        statuses,
        active_filter: filter.unwrap_or_default(),
    })
}

#[derive(Template)]
#[template(path = "admin/ticket_detail.html")]
struct AdminTicketDetailTemplate {
    site_name: String,
    ticket: Ticket,
    ticket_fa: String,
    messages: Vec<TicketMessage>,
}

pub async fn ticket_detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let ticket = support::find_ticket(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("تیکت".into()))?;
    let messages = support::ticket_messages(&s.db, id).await?;
    let ticket_fa = crate::filters::to_fa_digits(&ticket.ticket_number.to_string());
    render(AdminTicketDetailTemplate {
        site_name: s.site_name(),
        ticket,
        ticket_fa,
        messages,
    })
}

#[derive(Deserialize)]
pub struct AdminTicketReplyForm {
    pub body: String,
}

pub async fn ticket_reply(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<AdminTicketReplyForm>,
) -> AppResult<axum::response::Response> {
    if !f.body.trim().is_empty() {
        support::add_ticket_message(&s.db, id, "staff", f.body.trim(), "answered").await?;
        // اطلاع‌رسانی به مشتری (در صورت داشتن ایمیل).
        if let Ok(Some(ticket)) = support::find_ticket(&s.db, id).await {
            if let Some(email) = ticket.email.as_deref().filter(|e| e.contains('@')) {
                let body = format!(
                    "سلام،\n\nپشتیبانی زاگرس به تیکت شما (#{} — {}) پاسخ داد:\n\n{}\n\nبرای دیدن کامل گفت‌وگو به سایت مراجعه کنید.\n\nزاگرس — جهان کولاک",
                    ticket.ticket_number, ticket.subject, f.body.trim()
                );
                s.mailer
                    .send(
                        email,
                        &format!("پاسخ تیکت #{}", ticket.ticket_number),
                        &body,
                    )
                    .await;
            }
        }
    }
    Ok(Redirect::to(&format!("/admin/tickets/{}", id)).into_response())
}

#[derive(Deserialize)]
pub struct AdminTicketStatusForm {
    pub status: String,
}

pub async fn ticket_status(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<AdminTicketStatusForm>,
) -> AppResult<axum::response::Response> {
    support::set_ticket_status(&s.db, id, &f.status).await?;
    Ok(Redirect::to(&format!("/admin/tickets/{}", id)).into_response())
}

// ===================== پشتیبانی: چت زنده =====================

#[derive(Template)]
#[template(path = "admin/chats.html")]
struct AdminChatsTemplate {
    site_name: String,
    sessions: Vec<db::support::ChatSessionRow>,
    active_filter: String,
}

pub async fn chats(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<StatusQuery>,
) -> AppResult<Html<String>> {
    // پیش‌فرض «active» (مثل قبل)؛ «closed» بسته‌ها، «all» همه.
    let (db_filter, active_filter): (Option<&str>, &str) = match q.status.as_deref() {
        Some("closed") => (Some("closed"), "closed"),
        Some("all") => (None, "all"),
        _ => (Some("active"), "active"),
    };
    let sessions = support::list_chat_sessions(&s.db, db_filter).await?;
    render(AdminChatsTemplate {
        site_name: s.site_name(),
        sessions,
        active_filter: active_filter.to_string(),
    })
}

pub async fn close_chat(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<axum::response::Response> {
    support::close_chat_session(&s.db, id).await?;
    Ok(Redirect::to("/admin/chat").into_response())
}

#[derive(Template)]
#[template(path = "support/chat_room.html")]
struct AdminChatRoomTemplate {
    site_name: String,
    session_id: String,
    role: &'static str,
    peer_label: String,
}

pub async fn chat_room(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let session = support::find_chat_session(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("جلسه‌ی چت".into()))?;
    render(AdminChatRoomTemplate {
        site_name: s.site_name(),
        session_id: id.to_string(),
        role: "staff",
        peer_label: session.display_name(),
    })
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

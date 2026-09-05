//! روت‌های پشتیبانی مشتری: تیکت + چت زنده‌ی WebSocket (فاز ۴).

use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use chrono::{DateTime, Utc};
use db::support::{self, NewTicket, Ticket, TicketMessage};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

// ===================== مرکز پشتیبانی =====================

#[derive(Template)]
#[template(path = "support/hub.html")]
struct HubTemplate {
    site_name: String,
    logged_in: bool,
    prefill_name: String,
    prefill_email: String,
    my_tickets: Vec<Ticket>,
}

pub async fn hub(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let my_tickets = match &user {
        Some(u) => support::list_tickets_for_user(&s.db, u.id).await?,
        None => Vec::new(),
    };
    let (name, email) = match &user {
        Some(u) => (
            u.full_name.clone().unwrap_or_default(),
            u.email.clone().unwrap_or_default(),
        ),
        None => (String::new(), String::new()),
    };
    render(HubTemplate {
        site_name: s.site_name(),
        logged_in: user.is_some(),
        prefill_name: name,
        prefill_email: email,
        my_tickets,
    })
}

// ===================== ثبت تیکت =====================

#[derive(Deserialize)]
pub struct TicketForm {
    pub full_name: String,
    pub email: Option<String>,
    pub subject: String,
    pub department: String,
    pub priority: String,
    pub body: String,
}

const DEPARTMENTS: [&str; 4] = ["general", "sales", "technical", "billing"];
const PRIORITIES: [&str; 3] = ["low", "normal", "high"];

pub async fn create_ticket(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<TicketForm>,
) -> AppResult<Response> {
    if f.full_name.trim().is_empty() || f.subject.trim().is_empty() || f.body.trim().is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "نام، موضوع و متن پیام الزامی است.".into(),
        )));
    }
    let department = if DEPARTMENTS.contains(&f.department.as_str()) {
        f.department
    } else {
        "general".to_string()
    };
    let priority = if PRIORITIES.contains(&f.priority.as_str()) {
        f.priority
    } else {
        "normal".to_string()
    };

    let new = NewTicket {
        user_id: user.as_ref().map(|u| u.id),
        full_name: f.full_name.trim().to_string(),
        email: f.email.filter(|e| !e.trim().is_empty()),
        subject: f.subject.trim().to_string(),
        department,
        priority,
        body: f.body.trim().to_string(),
    };
    let ticket = support::create_ticket(&s.db, &new).await?;
    Ok(Redirect::to(&format!("/support/ticket/{}", ticket.ticket_number)).into_response())
}

// ===================== نمایش رشته‌ی تیکت =====================

#[derive(Template)]
#[template(path = "support/ticket.html")]
struct TicketThreadTemplate {
    site_name: String,
    ticket: Ticket,
    ticket_fa: String,
    messages: Vec<TicketMessage>,
}

pub async fn ticket_thread(
    State(s): State<AppState>,
    Path(number): Path<i64>,
) -> AppResult<Html<String>> {
    let ticket = support::find_ticket_by_number(&s.db, number)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("تیکت".into()))?;
    let messages = support::ticket_messages(&s.db, ticket.id).await?;
    let ticket_fa = crate::filters::to_fa_digits(&ticket.ticket_number.to_string());
    render(TicketThreadTemplate {
        site_name: s.site_name(),
        ticket,
        ticket_fa,
        messages,
    })
}

#[derive(Deserialize)]
pub struct LookupForm {
    pub ticket: String,
}

/// پیگیری تیکت با شماره (برای مهمان) → ریدایرکت به رشته‌ی تیکت.
pub async fn ticket_lookup(Form(f): Form<LookupForm>) -> Response {
    let digits: String = f.ticket.chars().filter(|c| c.is_ascii_digit()).collect();
    Redirect::to(&format!("/support/ticket/{digits}")).into_response()
}

#[derive(Deserialize)]
pub struct ReplyForm {
    pub body: String,
}

pub async fn ticket_reply(
    State(s): State<AppState>,
    Path(number): Path<i64>,
    Form(f): Form<ReplyForm>,
) -> AppResult<Response> {
    let ticket = support::find_ticket_by_number(&s.db, number)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("تیکت".into()))?;
    if !ticket.is_closed() && !f.body.trim().is_empty() {
        support::add_ticket_message(&s.db, ticket.id, "customer", f.body.trim(), "open").await?;
    }
    Ok(Redirect::to(&format!("/support/ticket/{}", number)).into_response())
}

// ===================== چت زنده =====================

#[derive(Template)]
#[template(path = "support/chat_start.html")]
struct ChatStartTemplate {
    site_name: String,
    logged_in: bool,
    prefill_name: String,
}

pub async fn chat_start_page(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let prefill_name = user
        .as_ref()
        .and_then(|u| u.full_name.clone())
        .unwrap_or_default();
    render(ChatStartTemplate {
        site_name: s.site_name(),
        logged_in: user.is_some(),
        prefill_name,
    })
}

#[derive(Deserialize)]
pub struct ChatStartForm {
    pub guest_name: Option<String>,
}

pub async fn chat_start(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<ChatStartForm>,
) -> AppResult<Response> {
    // نام نمایشی: ورودی فرم، یا نام کاربر لاگین‌شده، یا موبایل.
    let name = f
        .guest_name
        .filter(|n| !n.trim().is_empty())
        .or_else(|| user.as_ref().and_then(|u| u.full_name.clone()))
        .or_else(|| user.as_ref().map(|u| u.mobile.clone()));
    let session = support::create_chat_session(&s.db, user.as_ref().map(|u| u.id), name.as_deref())
        .await?;
    Ok(Redirect::to(&format!("/support/chat/{}", session.id)).into_response())
}

#[derive(Template)]
#[template(path = "support/chat_room.html")]
struct ChatRoomTemplate {
    site_name: String,
    session_id: String,
    role: &'static str,
    peer_label: String,
}

pub async fn chat_room(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Html<String>> {
    let _session = support::find_chat_session(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("جلسه‌ی چت".into()))?;
    render(ChatRoomTemplate {
        site_name: s.site_name(),
        session_id: id.to_string(),
        role: "customer",
        peer_label: "پشتیبانی زاگرس".to_string(),
    })
}

/// ارتقای WebSocket چت. نقش از روی نشست تعیین می‌شود: ادمین = اپراتور، بقیه = مشتری.
pub async fn chat_ws(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    CurrentUser(user): CurrentUser,
    ws: WebSocketUpgrade,
) -> Response {
    let role = match &user {
        Some(u) if u.role == "admin" => "staff",
        _ => "customer",
    };
    ws.on_upgrade(move |socket| handle_chat(socket, s, id, role))
}

/// قالب‌بندی یک پیام چت به‌صورت JSON برای ارسال روی سیم.
fn chat_json(sender: &str, body: &str, at: DateTime<Utc>) -> String {
    serde_json::json!({
        "sender": sender,
        "body": body,
        "at": at.to_rfc3339(),
    })
    .to_string()
}

/// حلقه‌ی اصلی یک اتصال چت: تاریخچه را می‌فرستد، سپس پیام‌های ورودی را ذخیره/پخش
/// و پیام‌های پخش‌شده را به این سوکت می‌فرستد. تک‌تسک با `select!` (بدون split).
async fn handle_chat(mut socket: WebSocket, s: AppState, session_id: Uuid, role: &'static str) {
    let tx = s.chat.channel(session_id);
    // پیش از ارسال تاریخچه مشترک می‌شویم تا پیام‌های هم‌زمان از دست نروند.
    let mut rx = tx.subscribe();

    // ارسال تاریخچه‌ی گفت‌وگو.
    match support::chat_messages(&s.db, session_id).await {
        Ok(history) => {
            for m in history {
                let json = chat_json(&m.sender, &m.body, m.created_at);
                if socket.send(Message::Text(json.into())).await.is_err() {
                    return;
                }
            }
        }
        Err(e) => tracing::error!(error = %e, "خواندن تاریخچه‌ی چت شکست خورد"),
    }

    loop {
        tokio::select! {
            // پیام از سمت این سوکت (مشتری یا اپراتور).
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let body = text.as_str().trim();
                        if body.is_empty() {
                            continue;
                        }
                        match support::add_chat_message(&s.db, session_id, role, body).await {
                            Ok(m) => {
                                let _ = tx.send(chat_json(&m.sender, &m.body, m.created_at));
                            }
                            Err(e) => tracing::error!(error = %e, "ذخیره‌ی پیام چت شکست خورد"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(_)) => {} // ping/pong/binary نادیده.
                }
            }
            // پیامی که روی کانال جلسه پخش شده (از خود ما یا طرف مقابل).
            broadcasted = rx.recv() => {
                match broadcasted {
                    Ok(json) => {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue, // از پیام‌های جامانده می‌گذریم.
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

//! پنل ادمین: کارت هدیه (صدور/فهرست) و اعلان همگانی.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::parse_digits;
use crate::state::AppState;
use crate::web::render;
use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

// ===================== کارت هدیه =====================

#[derive(Template)]
#[template(path = "admin/giftcards.html")]
struct GiftCardsTemplate {
    site_name: String,
    cards: Vec<db::gift_cards::GiftCard>,
    flash: Option<String>,
}

#[derive(Deserialize)]
pub struct GiftQuery {
    pub created: Option<String>,
}

pub async fn giftcards(
    State(s): State<AppState>,
    _admin: AdminUser,
    axum::extract::Query(q): axum::extract::Query<GiftQuery>,
) -> AppResult<Html<String>> {
    let cards = db::gift_cards::list_all(&s.db, 200).await?;
    let flash = q.created.map(|c| format!("کارت هدیه با کد {c} ساخته شد."));
    render(GiftCardsTemplate {
        site_name: s.site_name(),
        cards,
        flash,
    })
}

#[derive(Deserialize)]
pub struct GiftCreateForm {
    pub amount_toman: String,
    pub days: Option<String>,
}

pub async fn create_giftcard(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<GiftCreateForm>,
) -> AppResult<Response> {
    let toman = parse_digits(f.amount_toman.trim()).unwrap_or(0);
    if toman <= 0 {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "مبلغ کارت هدیه نامعتبر است.".into(),
        )));
    }
    let code = crate::auth::generate_link_code();
    let expires = f
        .days
        .as_deref()
        .and_then(|d| parse_digits(d.trim()).ok())
        .filter(|d| *d > 0)
        .map(|d| chrono::Utc::now() + chrono::Duration::days(d));
    db::gift_cards::issue(&s.db, &code, toman * 10, None, expires).await?;
    Ok(Redirect::to(&format!("/admin/giftcards?created={code}")).into_response())
}

// ===================== اعلان همگانی =====================

#[derive(Template)]
#[template(path = "admin/notify.html")]
struct NotifyTemplate {
    site_name: String,
    sent: Option<String>,
}

#[derive(Deserialize)]
pub struct NotifyQuery {
    pub sent: Option<String>,
}

pub async fn notify_page(
    State(s): State<AppState>,
    _admin: AdminUser,
    axum::extract::Query(q): axum::extract::Query<NotifyQuery>,
) -> AppResult<Html<String>> {
    let sent = q.sent.map(|n| format!("اعلان برای {n} کاربر ارسال شد."));
    render(NotifyTemplate {
        site_name: s.site_name(),
        sent,
    })
}

#[derive(Deserialize)]
pub struct NotifyForm {
    pub title: String,
    pub body: Option<String>,
    pub url: Option<String>,
}

pub async fn notify_send(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<NotifyForm>,
) -> AppResult<Response> {
    let title = f.title.trim();
    if title.is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "عنوان اعلان الزامی است.".into(),
        )));
    }
    let body = f.body.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let url = f.url.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let n = db::notifications::broadcast(&s.db, "coupon", title, body, url).await?;
    Ok(Redirect::to(&format!("/admin/notify?sent={n}")).into_response())
}

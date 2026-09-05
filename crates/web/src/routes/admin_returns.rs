//! مدیریت مرجوعی (پنل ادمین) — فهرست، جزئیات، تغییر وضعیت/بازپرداخت.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::parse_digits;
use crate::state::AppState;
use crate::web::render;
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use uuid::Uuid;

struct StatusOption {
    key: &'static str,
    label: &'static str,
}
fn status_options() -> Vec<StatusOption> {
    db::returns::RETURN_STATUSES
        .iter()
        .map(|k| StatusOption {
            key: k,
            label: db::returns::status_label(k),
        })
        .collect()
}

#[derive(Template)]
#[template(path = "admin/returns.html")]
struct ReturnsTemplate {
    site_name: String,
    returns: Vec<db::returns::AdminReturnRow>,
    statuses: Vec<StatusOption>,
    active_filter: String,
}

#[derive(Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
}

pub async fn list(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<StatusQuery>,
) -> AppResult<Html<String>> {
    let filter = q.status.filter(|st| db::returns::RETURN_STATUSES.contains(&st.as_str()));
    let returns = db::returns::list_all(&s.db, filter.as_deref()).await?;
    render(ReturnsTemplate {
        site_name: s.site_name(),
        returns,
        statuses: status_options(),
        active_filter: filter.unwrap_or_default(),
    })
}

#[derive(Template)]
#[template(path = "admin/return_detail.html")]
struct ReturnDetailTemplate {
    site_name: String,
    ret: db::returns::Return,
    order: Option<db::orders::Order>,
    items: Vec<db::returns::ReturnItem>,
    history: Vec<db::returns::StatusLog>,
    statuses: Vec<StatusOption>,
    refund_toman: String,
}

pub async fn detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let ret = db::returns::find(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("مرجوعی".into()))?;
    let order = db::orders::find(&s.db, ret.order_id).await?;
    let items = db::returns::items(&s.db, ret.id).await?;
    let history = db::returns::history(&s.db, ret.id).await?;
    let refund_toman = ret.refund_amount.map(|a| (a / 10).to_string()).unwrap_or_default();
    render(ReturnDetailTemplate {
        site_name: s.site_name(),
        ret,
        order,
        items,
        history,
        statuses: status_options(),
        refund_toman,
    })
}

#[derive(Deserialize)]
pub struct UpdateForm {
    pub status: String,
    pub refund_toman: Option<String>,
    pub note: Option<String>,
}

pub async fn update_status(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<UpdateForm>,
) -> AppResult<Response> {
    if !db::returns::RETURN_STATUSES.contains(&f.status.as_str()) {
        return Err(AppError::Domain(domain::DomainError::Invalid("وضعیت نامعتبر".into())));
    }
    let refund = f
        .refund_toman
        .as_deref()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .and_then(|x| parse_digits(x).ok())
        .map(|t| t * 10);
    let note = f.note.as_deref().map(str::trim).filter(|x| !x.is_empty());
    db::returns::update_status(&s.db, id, &f.status, refund, note).await?;

    // مرجوعیِ «بازپرداخت‌شده»: مبلغ بازگشتی به کیف پول مشتری واریز می‌شود + اعلان.
    if f.status == "refunded" {
        if let Ok(Some(ret)) = db::returns::find(&s.db, id).await {
            if let (Some(uid), Some(amount)) = (ret.user_id, ret.refund_amount.filter(|a| *a > 0)) {
                let num = crate::filters::to_fa_digits(&ret.return_number.to_string());
                let rnote = format!("بازپرداخت مرجوعی #{num}");
                if db::wallet::add_transaction(&s.db, uid, amount, "return_refund", Some(ret.order_id), Some(&rnote)).await.is_ok() {
                    let title = format!("بازپرداخت مرجوعی #{num} به کیف پول شما واریز شد");
                    let _ = db::notifications::create(&s.db, uid, "order", &title, None, Some("/account/wallet")).await;
                }
            }
        }
    }
    Ok(Redirect::to(&format!("/admin/returns/{id}")).into_response())
}

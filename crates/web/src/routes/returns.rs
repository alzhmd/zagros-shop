//! مرجوعی/بازگشت کالا — درخواست، فهرست و جزئیات برای مشتری (فاز ارتقا).

use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use crate::web::render;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

const REASONS: [(&str, &str); 5] = [
    ("defective", "کالای معیوب/خراب"),
    ("wrong", "ارسال کالای اشتباه"),
    ("not_as_described", "مغایرت با توضیحات"),
    ("changed_mind", "انصراف از خرید"),
    ("other", "سایر"),
];

/// سفارش مرجوعی‌پذیر است اگر تحویل شده باشد (و مرجوعی باز نداشته باشد).
fn is_returnable(status: &str) -> bool {
    matches!(status, "delivered")
}

// ---------- فرم درخواست ----------

#[derive(Template)]
#[template(path = "account/return_form.html")]
struct ReturnFormTemplate {
    site_name: String,
    order: db::orders::Order,
    order_number_fa: String,
    items: Vec<db::orders::OrderItemFull>,
    reasons: Vec<(&'static str, &'static str)>,
}

pub async fn request_form(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(order_id): Path<Uuid>,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(order) = db::orders::find(&s.db, order_id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "سفارش".into(),
        )));
    };
    if order.user_id != Some(user.id) {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    if !is_returnable(&order.status) {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "فقط برای سفارش‌های تحویل‌شده می‌توان درخواست مرجوعی ثبت کرد.".into(),
        )));
    }
    if db::returns::exists_open_for_order(&s.db, order.id).await? {
        return Ok(Redirect::to("/account/returns").into_response());
    }
    let items = db::orders::items_with_id(&s.db, order.id).await?;
    let order_number_fa = crate::filters::to_fa_digits(&order.order_number.to_string());
    Ok(render(ReturnFormTemplate {
        site_name: s.site_name(),
        order,
        order_number_fa,
        items,
        reasons: REASONS.to_vec(),
    })?
    .into_response())
}

pub async fn submit(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(order_id): Path<Uuid>,
    Form(pairs): Form<Vec<(String, String)>>,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(order) = db::orders::find(&s.db, order_id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "سفارش".into(),
        )));
    };
    if order.user_id != Some(user.id) || !is_returnable(&order.status) {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    if db::returns::exists_open_for_order(&s.db, order.id).await? {
        return Ok(Redirect::to("/account/returns").into_response());
    }

    // تجزیه‌ی فرم: item_<uuid>=on , qty_<uuid>=N , reason, description.
    let mut map: HashMap<String, String> = HashMap::new();
    for (k, v) in pairs {
        map.insert(k, v);
    }
    let reason = map.get("reason").cloned().unwrap_or_else(|| "other".into());
    let reason = if db::returns::RETURN_REASONS.contains(&reason.as_str()) {
        reason
    } else {
        "other".into()
    };
    let description = map
        .get("description")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let order_items = db::orders::items_with_id(&s.db, order.id).await?;
    let mut selected: Vec<db::returns::NewReturnItem> = Vec::new();
    for oi in &order_items {
        let key = format!("item_{}", oi.id);
        if map.get(&key).map(|v| v == "on").unwrap_or(false) {
            // مقدار: از qty_<id>، محدود به مقدار خریداری‌شده.
            let qty = map
                .get(&format!("qty_{}", oi.id))
                .and_then(|q| q.trim().parse::<i32>().ok())
                .unwrap_or(oi.qty)
                .clamp(1, oi.qty);
            selected.push(db::returns::NewReturnItem {
                order_item_id: oi.id,
                product_name: oi.product_name.clone(),
                variant_title: oi.variant_title.clone(),
                qty,
                unit_price: oi.unit_price,
            });
        }
    }
    if selected.is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "حداقل یک کالا برای مرجوعی انتخاب کنید.".into(),
        )));
    }

    let ret = db::returns::create(
        &s.db,
        order.id,
        Some(user.id),
        &reason,
        description.as_deref(),
        &selected,
    )
    .await?;
    Ok(Redirect::to(&format!("/return/{}", ret.return_number)).into_response())
}

// ---------- فهرست مرجوعی‌های من ----------

#[derive(Template)]
#[template(path = "account/returns.html")]
struct ReturnsTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    returns: Vec<db::returns::Return>,
}

pub async fn list(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let returns = db::returns::list_for_user(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    Ok(render(ReturnsTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        returns,
    })?
    .into_response())
}

// ---------- جزئیات مرجوعی ----------

#[derive(Template)]
#[template(path = "account/return_detail.html")]
struct ReturnDetailTemplate {
    site_name: String,
    ret: db::returns::Return,
    items: Vec<db::returns::ReturnItem>,
    history: Vec<db::returns::StatusLog>,
}

pub async fn detail(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(number): Path<i64>,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(ret) = db::returns::find_by_number(&s.db, number).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "مرجوعی".into(),
        )));
    };
    if ret.user_id != Some(user.id) {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    let items = db::returns::items(&s.db, ret.id).await?;
    let history = db::returns::history(&s.db, ret.id).await?;
    Ok(render(ReturnDetailTemplate {
        site_name: s.site_name(),
        ret,
        items,
        history,
    })?
    .into_response())
}

#[derive(Deserialize)]
pub struct CancelForm {
    pub id: Uuid,
}

pub async fn cancel(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<CancelForm>,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    db::returns::cancel_by_user(&s.db, f.id, user.id).await?;
    Ok(Redirect::to("/account/returns").into_response())
}

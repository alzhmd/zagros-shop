//! ثبت نقد و امتیاز محصول (فاز ارتقا).

use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReviewForm {
    pub product_id: Uuid,
    pub slug: String,
    pub rating: i32,
    pub title: Option<String>,
    pub body: String,
    pub author_name: Option<String>,
}

pub async fn submit(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<ReviewForm>,
) -> AppResult<Response> {
    let rating = f.rating.clamp(1, 5);
    let body = f.body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "متن نقد را بنویسید.".into(),
        )));
    }
    // نام نویسنده: نام کاربر لاگین‌شده، یا ورودی فرم، یا «کاربر مهمان».
    let author_name = user
        .as_ref()
        .and_then(|u| u.full_name.clone())
        .filter(|n| !n.trim().is_empty())
        .or_else(|| f.author_name.clone().filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| "کاربر مهمان".to_string());

    let is_verified = match &user {
        Some(u) => db::reviews::has_purchased(&s.db, u.id, f.product_id)
            .await
            .unwrap_or(false),
        None => false,
    };

    let new = db::reviews::NewReview {
        product_id: f.product_id,
        user_id: user.as_ref().map(|u| u.id),
        author_name,
        rating,
        title: f
            .title
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty()),
        body,
        is_verified,
    };
    db::reviews::upsert(&s.db, &new).await?;

    Ok(Redirect::to(&format!("/product/{}?review=ok", f.slug)).into_response())
}

// ---------- پرسش محصول ----------

#[derive(Deserialize)]
pub struct QuestionForm {
    pub product_id: Uuid,
    pub slug: String,
    pub body: String,
    pub author_name: Option<String>,
}

pub async fn ask(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<QuestionForm>,
) -> AppResult<Response> {
    let body = f.body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "متن پرسش را بنویسید.".into(),
        )));
    }
    let author_name = user
        .as_ref()
        .and_then(|u| u.full_name.clone())
        .filter(|n| !n.trim().is_empty())
        .or_else(|| f.author_name.clone().filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| "کاربر مهمان".to_string());
    db::qa::ask(
        &s.db,
        f.product_id,
        user.as_ref().map(|u| u.id),
        &author_name,
        &body,
    )
    .await?;
    Ok(Redirect::to(&format!("/product/{}?q=ok", f.slug)).into_response())
}

// ---------- رأی «مفید بود» (HTMX) ----------

/// ثبت «این نقد مفید بود» — یک رأی به‌ازای هر کاربر؛ پاسخ، دکمه‌ی به‌روزشده است.
pub async fn helpful(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let Some(user) = user else {
        // مهمان: به ورود هدایت شود.
        return Ok(Redirect::to("/account/login").into_response());
    };
    let count = db::reviews::mark_helpful(&s.db, id, user.id).await?;
    let fa = crate::filters::to_fa_digits(&count.to_string());
    Ok(Html(format!(
        "<span class=\"review-helpful is-voted\">👍 مفید بود · {fa}</span>"
    ))
    .into_response())
}

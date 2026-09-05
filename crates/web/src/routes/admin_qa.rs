//! مدیریت پرسش و پاسخ محصول (پنل ادمین) — پاسخ/تأیید/رد/حذف.

use crate::error::AppResult;
use crate::extract::AdminUser;
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

#[derive(Template)]
#[template(path = "admin/questions.html")]
struct QuestionsTemplate {
    site_name: String,
    questions: Vec<db::qa::AdminQuestionRow>,
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
    let valid = ["pending", "approved", "rejected"];
    let filter = q.status.filter(|st| valid.contains(&st.as_str()));
    let questions = db::qa::list_all(&s.db, filter.as_deref()).await?;
    render(QuestionsTemplate {
        site_name: s.site_name(),
        questions,
        active_filter: filter.unwrap_or_default(),
    })
}

#[derive(Deserialize)]
pub struct AnswerForm {
    pub body: String,
}

pub async fn answer(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<AnswerForm>,
) -> AppResult<Response> {
    let body = f.body.trim();
    if !body.is_empty() {
        db::qa::answer(&s.db, id, body).await?;
        // اعلان پاسخ به پرسنده.
        if let Ok(Some((Some(uid), name, slug))) = db::qa::notify_info(&s.db, id).await {
            let title = format!("به پرسش شما درباره‌ی «{name}» پاسخ داده شد");
            let url = format!("/product/{slug}#qa");
            let _ = db::notifications::create(&s.db, uid, "review", &title, None, Some(&url)).await;
        }
    }
    Ok(Redirect::to("/admin/questions").into_response())
}

#[derive(Deserialize)]
pub struct StatusForm {
    pub status: String,
}

pub async fn set_status(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<StatusForm>,
) -> AppResult<Response> {
    let valid = ["pending", "approved", "rejected"];
    if valid.contains(&f.status.as_str()) {
        db::qa::set_status(&s.db, id, &f.status).await?;
    }
    Ok(Redirect::to("/admin/questions").into_response())
}

pub async fn delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    db::qa::delete(&s.db, id).await?;
    Ok(Redirect::to("/admin/questions").into_response())
}

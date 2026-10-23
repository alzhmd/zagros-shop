//! مدیریت نقد و امتیاز محصول (پنل ادمین) — تأیید/رد/حذف.

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

const STATUSES: [(&str, &str); 3] = [
    ("pending", "در انتظار"),
    ("approved", "تأییدشده"),
    ("rejected", "ردشده"),
];

#[derive(Template)]
#[template(path = "admin/reviews.html")]
struct ReviewsTemplate {
    site_name: String,
    reviews: Vec<db::reviews::AdminReviewRow>,
    statuses: Vec<(&'static str, &'static str)>,
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
    let reviews = db::reviews::list_all(&s.db, filter.as_deref()).await?;
    render(ReviewsTemplate {
        site_name: s.site_name(),
        reviews,
        statuses: STATUSES.to_vec(),
        active_filter: filter.unwrap_or_default(),
    })
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
        db::reviews::set_status(&s.db, id, &f.status).await?;
        // اعلان به نویسنده هنگام تأیید نقد.
        if f.status == "approved" {
            if let Ok(Some((Some(uid), name, slug))) = db::reviews::notify_info(&s.db, id).await {
                let title = format!("نظر شما برای «{name}» تأیید شد");
                let url = format!("/product/{slug}");
                let _ =
                    db::notifications::create(&s.db, uid, "review", &title, None, Some(&url)).await;
            }
        }
    }
    Ok(Redirect::to("/admin/reviews").into_response())
}

pub async fn delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    db::reviews::delete(&s.db, id).await?;
    Ok(Redirect::to("/admin/reviews").into_response())
}

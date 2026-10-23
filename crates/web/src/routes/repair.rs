use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use db::repairs::{NewRepair, RepairRequest, StatusLog};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "repair/form.html")]
struct FormTemplate {
    site_name: String,
    prefill_name: String,
    prefill_phone: String,
}

pub async fn form(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let (name, phone) = match &user {
        Some(u) => (u.full_name.clone().unwrap_or_default(), u.mobile.clone()),
        None => (String::new(), String::new()),
    };
    render(FormTemplate {
        site_name: s.site_name(),
        prefill_name: name,
        prefill_phone: phone,
    })
}

#[derive(Deserialize)]
pub struct RepairForm {
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

pub async fn submit(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<RepairForm>,
) -> AppResult<axum::response::Response> {
    if f.full_name.trim().is_empty() || f.problem.trim().is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "نام و شرح مشکل الزامی است.".into(),
        )));
    }
    if f.method == "onsite"
        && (f.address.as_deref().unwrap_or("").trim().is_empty()
            || f.province.as_deref().unwrap_or("").trim().is_empty())
    {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "برای اعزام کارشناس، استان و نشانی الزامی است.".into(),
        )));
    }

    let new = NewRepair {
        user_id: user.as_ref().map(|u| u.id),
        full_name: f.full_name,
        phone: f.phone,
        device_type: f.device_type,
        brand: f.brand.filter(|s| !s.trim().is_empty()),
        model: f.model.filter(|s| !s.trim().is_empty()),
        problem: f.problem,
        method: if f.method == "onsite" {
            "onsite"
        } else {
            "dropoff"
        }
        .to_string(),
        province: f.province.filter(|s| !s.trim().is_empty()),
        city: f.city.filter(|s| !s.trim().is_empty()),
        address: f.address.filter(|s| !s.trim().is_empty()),
    };

    let req = db::repairs::create(&s.db, &new).await?;
    Ok(Redirect::to(&format!("/repair/track/{}", req.ticket_number)).into_response())
}

#[derive(Template)]
#[template(path = "repair/track.html")]
struct TrackTemplate {
    site_name: String,
    repair: RepairRequest,
    ticket_fa: String,
    history: Vec<StatusLog>,
    cost_estimate_fa: Option<String>,
}

pub async fn track(State(s): State<AppState>, Path(ticket): Path<i64>) -> AppResult<Html<String>> {
    let repair = db::repairs::find_by_ticket(&s.db, ticket)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("درخواست تعمیر".into()))?;
    let history = db::repairs::status_history(&s.db, repair.id).await?;
    let ticket_fa = crate::filters::to_fa_digits(&repair.ticket_number.to_string());
    let cost_estimate_fa = repair
        .cost_estimate
        .map(|c| domain::Rial(c).format_toman_fa());

    render(TrackTemplate {
        site_name: s.site_name(),
        repair,
        ticket_fa,
        history,
        cost_estimate_fa,
    })
}

#[derive(Template)]
#[template(path = "repair/lookup.html")]
struct LookupTemplate {
    site_name: String,
}

pub async fn lookup_page(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(LookupTemplate {
        site_name: s.site_name(),
    })
}

#[derive(Deserialize)]
pub struct LookupForm {
    pub ticket: String,
}

pub async fn lookup(
    State(_s): State<AppState>,
    Form(f): Form<LookupForm>,
) -> AppResult<axum::response::Response> {
    let digits: String = f.ticket.chars().filter(|c| c.is_ascii_digit()).collect();
    Ok(Redirect::to(&format!("/repair/track/{}", digits)).into_response())
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

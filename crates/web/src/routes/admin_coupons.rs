//! روت‌های مدیریت کد تخفیف (پنل ادمین).
//! مقادیر پولی به تومان وارد می‌شوند و ×۱۰ به ریال ذخیره می‌شوند؛
//! برای نوع «درصدی»، مقدار همان درصد است (تبدیل نمی‌شود).

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::{checkbox, clean_slug, parse_digits};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use chrono::{NaiveDate, TimeZone, Utc};
use db::coupons::{self, Coupon, CouponInput};
use serde::Deserialize;
use uuid::Uuid;

// ===================== فهرست =====================

#[derive(Template)]
#[template(path = "admin/coupons.html")]
struct CouponsTemplate {
    site_name: String,
    coupons: Vec<Coupon>,
}

pub async fn list(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let coupons = coupons::list_all(&s.db).await?;
    render(CouponsTemplate {
        site_name: s.site_name(),
        coupons,
    })
}

// ===================== فرم ساخت/ویرایش =====================

#[derive(Template)]
#[template(path = "admin/coupon_form.html")]
struct CouponFormTemplate {
    site_name: String,
    is_edit: bool,
    id: String,
    code: String,
    description: String,
    kind: String,
    value: String,        // درصد یا تومان (بسته به kind)
    min_order_toman: String,
    max_discount_toman: String,
    usage_limit: String,
    per_user_limit: String,
    expires_at: String,   // YYYY-MM-DD
    is_active: bool,
    used_count: i32,
    error: Option<String>,
}

fn empty_form(site_name: String, error: Option<String>) -> CouponFormTemplate {
    CouponFormTemplate {
        site_name,
        is_edit: false,
        id: String::new(),
        code: String::new(),
        description: String::new(),
        kind: "percent".into(),
        value: String::new(),
        min_order_toman: "0".into(),
        max_discount_toman: String::new(),
        usage_limit: String::new(),
        per_user_limit: "1".into(),
        expires_at: String::new(),
        is_active: true,
        used_count: 0,
        error,
    }
}

pub async fn new_form(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    render(empty_form(s.site_name(), None))
}

fn edit_form_from(c: &Coupon, site_name: String, error: Option<String>) -> CouponFormTemplate {
    // مقدار: درصد همان است؛ ثابت به تومان.
    let value = if c.is_percent() {
        c.value.to_string()
    } else {
        (c.value / 10).to_string()
    };
    CouponFormTemplate {
        site_name,
        is_edit: true,
        id: c.id.to_string(),
        code: c.code.clone(),
        description: c.description.clone().unwrap_or_default(),
        kind: c.kind.clone(),
        value,
        min_order_toman: (c.min_order / 10).to_string(),
        max_discount_toman: c.max_discount.map(|m| (m / 10).to_string()).unwrap_or_default(),
        usage_limit: c.usage_limit.map(|u| u.to_string()).unwrap_or_default(),
        per_user_limit: c.per_user_limit.to_string(),
        expires_at: c.expires_at.map(|e| e.format("%Y-%m-%d").to_string()).unwrap_or_default(),
        is_active: c.is_active,
        used_count: c.used_count,
        error,
    }
}

pub async fn edit_form(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let c = coupons::find_by_id(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("کد تخفیف".into()))?;
    render(edit_form_from(&c, s.site_name(), None))
}

// ===================== تجزیه‌ی فرم =====================

#[derive(Deserialize)]
pub struct CouponForm {
    pub code: String,
    pub description: Option<String>,
    pub kind: String,
    pub value: String,
    pub min_order_toman: Option<String>,
    pub max_discount_toman: Option<String>,
    pub usage_limit: Option<String>,
    pub per_user_limit: Option<String>,
    pub expires_at: Option<String>,
    pub is_active: Option<String>,
}

/// تومان → ریال؛ ورودی خالی → None.
fn toman_to_rial_opt(v: &Option<String>) -> Result<Option<i64>, String> {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Ok(Some(parse_digits(s)? * 10)),
        None => Ok(None),
    }
}

fn int_opt(v: &Option<String>) -> Result<Option<i32>, String> {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Ok(Some(parse_digits(s)? as i32)),
        None => Ok(None),
    }
}

/// تاریخ YYYY-MM-DD → پایان آن روز (UTC). خالی → None.
fn parse_date_opt(v: &Option<String>) -> Result<Option<chrono::DateTime<Utc>>, String> {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => {
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| "تاریخ نامعتبر است (YYYY-MM-DD).".to_string())?;
            let dt = d.and_hms_opt(23, 59, 59).unwrap();
            Ok(Some(Utc.from_utc_datetime(&dt)))
        }
        None => Ok(None),
    }
}

fn parse_coupon_form(f: &CouponForm) -> Result<CouponInput, String> {
    let code = clean_slug(&f.code).to_uppercase();
    if code.is_empty() {
        return Err("کد تخفیف باید حروف/رقم انگلیسی باشد.".into());
    }
    let kind = if f.kind == "fixed" { "fixed" } else { "percent" };
    let raw_value = parse_digits(f.value.trim())?;
    if raw_value <= 0 {
        return Err("مقدار تخفیف باید بزرگ‌تر از صفر باشد.".into());
    }
    // درصدی: ۱..۱۰۰ ؛ ثابت: تومان → ریال.
    let value = if kind == "percent" {
        if raw_value > 100 {
            return Err("درصد تخفیف نمی‌تواند بیش از ۱۰۰ باشد.".into());
        }
        raw_value
    } else {
        raw_value * 10
    };
    let min_order = toman_to_rial_opt(&f.min_order_toman)?.unwrap_or(0);
    let max_discount = toman_to_rial_opt(&f.max_discount_toman)?;
    let usage_limit = int_opt(&f.usage_limit)?;
    let per_user_limit = int_opt(&f.per_user_limit)?.unwrap_or(1).max(1);
    let expires_at = parse_date_opt(&f.expires_at)?;

    Ok(CouponInput {
        code,
        description: f.description.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        kind: kind.to_string(),
        value,
        min_order,
        max_discount,
        usage_limit,
        per_user_limit,
        expires_at,
        is_active: checkbox(&f.is_active),
    })
}

pub async fn create(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<CouponForm>,
) -> AppResult<Response> {
    let input = match parse_coupon_form(&f) {
        Ok(i) => i,
        Err(msg) => return Ok(render(empty_form(s.site_name(), Some(msg)))?.into_response()),
    };
    if coupons::code_exists(&s.db, &input.code, None).await? {
        let msg = format!("کد «{}» قبلاً وجود دارد.", input.code);
        return Ok(render(empty_form(s.site_name(), Some(msg)))?.into_response());
    }
    coupons::create(&s.db, &input).await?;
    Ok(Redirect::to("/admin/coupons").into_response())
}

pub async fn update(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<CouponForm>,
) -> AppResult<Response> {
    let Some(existing) = coupons::find_by_id(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("کد تخفیف".into())));
    };
    let input = match parse_coupon_form(&f) {
        Ok(i) => i,
        Err(msg) => {
            return Ok(render(edit_form_from(&existing, s.site_name(), Some(msg)))?
                .into_response())
        }
    };
    if coupons::code_exists(&s.db, &input.code, Some(id)).await? {
        let msg = format!("کد «{}» قبلاً وجود دارد.", input.code);
        return Ok(render(edit_form_from(&existing, s.site_name(), Some(msg)))?
            .into_response());
    }
    coupons::update(&s.db, id, &input).await?;
    Ok(Redirect::to("/admin/coupons").into_response())
}

#[derive(Deserialize)]
pub struct ToggleForm {
    pub value: Option<String>,
}

pub async fn toggle_active(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<ToggleForm>,
) -> AppResult<Response> {
    coupons::set_active(&s.db, id, checkbox(&f.value)).await?;
    Ok(Redirect::to("/admin/coupons").into_response())
}

pub async fn delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    coupons::delete(&s.db, id).await?;
    Ok(Redirect::to("/admin/coupons").into_response())
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

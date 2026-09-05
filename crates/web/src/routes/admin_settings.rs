//! پنل تنظیمات سایت (پنل ادمین) — مقادیر قابل‌ویرایش زنده در جدول settings.
//! رازها (توکن ربات، مرچنت، secret کوکی) اینجا ویرایش نمی‌شوند؛ فقط نمایش وضعیت.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::{checkbox, parse_digits};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "admin/settings.html")]
struct SettingsTemplate {
    site_name: String,
    // مقادیر فعلی (از کش تنظیمات).
    s_site_name: String,
    contact_phone: String,
    contact_address: String,
    shipping_toman: String,
    cod_province: String,
    rial_per_point: String,
    payments_enabled: bool,
    // روش‌های ارسال.
    ship_normal_toman: String,
    ship_normal_eta: String,
    ship_express_on: bool,
    ship_express_toman: String,
    ship_express_eta: String,
    ship_instant_on: bool,
    ship_instant_toman: String,
    ship_instant_eta: String,
    // باشگاه + کارت هدیه‌ی خوش‌آمد.
    silver_threshold: String,
    gold_threshold: String,
    gift_signup_on: bool,
    gift_signup_toman: String,
    gift_signup_days: String,
    // وضعیت ربات (فقط‌خواندنی).
    bot_active: bool,
    bot_username: String,
    saved: bool,
}

fn build(s: &AppState, saved: bool) -> SettingsTemplate {
    let g = |k: &str, d: &str| s.settings.get_or(k, d);
    let gi = |k: &str, d: i64| s.settings.get_i64(k, d).to_string();
    let gb = |k: &str, d: bool| s.settings.get_bool(k, d);
    SettingsTemplate {
        site_name: s.site_name(),
        s_site_name: s.settings.site_name(),
        contact_phone: s.settings.contact_phone(),
        contact_address: s.settings.contact_address(),
        shipping_toman: s.settings.shipping_toman().to_string(),
        cod_province: s.settings.cod_province(),
        rial_per_point: s.settings.rial_per_point().to_string(),
        payments_enabled: s.settings.payments_enabled(),
        ship_normal_toman: gi("shipping.normal.toman", 0),
        ship_normal_eta: g("shipping.normal.eta", "۳ تا ۵ روز کاری"),
        ship_express_on: gb("shipping.express.enabled", true),
        ship_express_toman: gi("shipping.express.toman", 50_000),
        ship_express_eta: g("shipping.express.eta", "۱ تا ۲ روز کاری"),
        ship_instant_on: gb("shipping.instant.enabled", false),
        ship_instant_toman: gi("shipping.instant.toman", 120_000),
        ship_instant_eta: g("shipping.instant.eta", "همان روز — فقط زنجان"),
        silver_threshold: gi("club.silver_threshold", 200),
        gold_threshold: gi("club.gold_threshold", 1000),
        gift_signup_on: gb("giftcard.signup_enabled", false),
        gift_signup_toman: gi("giftcard.signup_toman", 50_000),
        gift_signup_days: gi("giftcard.signup_days", 90),
        bot_active: s.bot.is_some(),
        bot_username: s.bot_username.to_string(),
        saved,
    }
}

#[derive(Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    pub contact_phone: Option<String>,
    pub contact_address: Option<String>,
    pub shipping_toman: Option<String>,
    pub cod_province: String,
    pub rial_per_point: Option<String>,
    pub payments_enabled: Option<String>,
    // روش‌های ارسال.
    pub ship_normal_toman: Option<String>,
    pub ship_normal_eta: Option<String>,
    pub ship_express_on: Option<String>,
    pub ship_express_toman: Option<String>,
    pub ship_express_eta: Option<String>,
    pub ship_instant_on: Option<String>,
    pub ship_instant_toman: Option<String>,
    pub ship_instant_eta: Option<String>,
    // باشگاه + کارت هدیه.
    pub silver_threshold: Option<String>,
    pub gold_threshold: Option<String>,
    pub gift_signup_on: Option<String>,
    pub gift_signup_toman: Option<String>,
    pub gift_signup_days: Option<String>,
}

pub async fn save(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<SettingsForm>,
) -> AppResult<Response> {
    let site_name = f.site_name.trim();
    if site_name.is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "نام سایت الزامی است.".into(),
        )));
    }
    let cod_province = f.cod_province.trim();
    let shipping = parse_digits(f.shipping_toman.as_deref().unwrap_or("0").trim()).unwrap_or(0);
    let rate = parse_digits(f.rial_per_point.as_deref().unwrap_or("100000").trim()).unwrap_or(100_000);

    // نوشتن کلید‌ها در جدول settings.
    let pairs: [(&str, String); 6] = [
        ("site.name", site_name.to_string()),
        ("site.contact_phone", f.contact_phone.unwrap_or_default().trim().to_string()),
        ("site.contact_address", f.contact_address.unwrap_or_default().trim().to_string()),
        ("shipping.flat_toman", shipping.max(0).to_string()),
        ("cod.province", cod_province.to_string()),
        ("loyalty.rial_per_point", rate.max(1).to_string()),
    ];
    for (k, v) in &pairs {
        db::settings::set(&s.db, k, v).await?;
    }
    // تاگل پرداخت (چک‌باکس).
    db::settings::set(
        &s.db,
        "payments.enabled",
        if checkbox(&f.payments_enabled) { "true" } else { "false" },
    )
    .await?;

    // روش‌های ارسال + باشگاه + کارت هدیه.
    let num = |o: &Option<String>, d: i64| {
        parse_digits(o.as_deref().unwrap_or("").trim()).unwrap_or(d).max(0).to_string()
    };
    let txt = |o: &Option<String>| o.clone().unwrap_or_default().trim().to_string();
    let extra: [(&str, String); 13] = [
        ("shipping.normal.toman", num(&f.ship_normal_toman, 0)),
        ("shipping.normal.eta", txt(&f.ship_normal_eta)),
        ("shipping.express.enabled", if checkbox(&f.ship_express_on) { "true" } else { "false" }.to_string()),
        ("shipping.express.toman", num(&f.ship_express_toman, 50_000)),
        ("shipping.express.eta", txt(&f.ship_express_eta)),
        ("shipping.instant.enabled", if checkbox(&f.ship_instant_on) { "true" } else { "false" }.to_string()),
        ("shipping.instant.toman", num(&f.ship_instant_toman, 120_000)),
        ("shipping.instant.eta", txt(&f.ship_instant_eta)),
        ("club.silver_threshold", num(&f.silver_threshold, 200)),
        ("club.gold_threshold", num(&f.gold_threshold, 1000)),
        ("giftcard.signup_enabled", if checkbox(&f.gift_signup_on) { "true" } else { "false" }.to_string()),
        ("giftcard.signup_toman", num(&f.gift_signup_toman, 50_000)),
        ("giftcard.signup_days", num(&f.gift_signup_days, 90)),
    ];
    for (k, v) in &extra {
        db::settings::set(&s.db, k, v).await?;
    }

    // تازه‌سازی کش زنده.
    s.settings.reload(&s.db).await;
    Ok(Redirect::to("/admin/settings?saved=1").into_response())
}

#[derive(Deserialize)]
pub struct SavedQuery {
    pub saved: Option<String>,
}

pub async fn page_with_flash(
    State(s): State<AppState>,
    _admin: AdminUser,
    axum::extract::Query(q): axum::extract::Query<SavedQuery>,
) -> AppResult<Html<String>> {
    render(build(&s, q.saved.as_deref() == Some("1")))
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

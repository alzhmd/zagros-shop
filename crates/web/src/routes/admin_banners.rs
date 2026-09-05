//! مدیریت بنر/کمپین (پنل ادمین) — اسلایدر صفحه‌ی اصلی و نوار تبلیغ.

use crate::error::AppResult;
use crate::extract::AdminUser;
use crate::routes::admin_util::parse_digits;
use crate::state::AppState;
use crate::web::render;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use db::banners::{Banner, BannerInput};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "admin/banners.html")]
struct BannersTemplate {
    site_name: String,
    banners: Vec<Banner>,
}

pub async fn list(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let banners = db::banners::list_all(&s.db).await?;
    render(BannersTemplate {
        site_name: s.site_name(),
        banners,
    })
}

#[derive(Template)]
#[template(path = "admin/banner_form.html")]
struct BannerFormTemplate {
    site_name: String,
    is_edit: bool,
    id: String,
    placement: String,
    title: String,
    subtitle: String,
    image_path: String,
    link: String,
    cta_label: String,
    sort_order: i32,
    is_active: bool,
}

pub async fn new_form(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    render(BannerFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        placement: "hero".into(),
        title: String::new(),
        subtitle: String::new(),
        image_path: String::new(),
        link: String::new(),
        cta_label: String::new(),
        sort_order: 0,
        is_active: true,
    })
}

pub async fn edit_form(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let b = db::banners::find(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("بنر".into()))?;
    render(BannerFormTemplate {
        site_name: s.site_name(),
        is_edit: true,
        id: b.id.to_string(),
        placement: b.placement,
        title: b.title.unwrap_or_default(),
        subtitle: b.subtitle.unwrap_or_default(),
        image_path: b.image_path.unwrap_or_default(),
        link: b.link.unwrap_or_default(),
        cta_label: b.cta_label.unwrap_or_default(),
        sort_order: b.sort_order,
        is_active: b.is_active,
    })
}

#[derive(Deserialize)]
pub struct BannerForm {
    pub placement: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub image_path: Option<String>,
    pub link: Option<String>,
    pub cta_label: Option<String>,
    pub sort_order: Option<String>,
    pub is_active: Option<String>,
}

fn parse(f: &BannerForm) -> BannerInput {
    let opt = |o: &Option<String>| o.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    BannerInput {
        placement: if f.placement.trim().is_empty() { "hero".into() } else { f.placement.trim().to_string() },
        title: opt(&f.title),
        subtitle: opt(&f.subtitle),
        image_path: opt(&f.image_path),
        link: opt(&f.link),
        cta_label: opt(&f.cta_label),
        bg_color: None,
        sort_order: f.sort_order.as_deref().and_then(|v| parse_digits(v.trim()).ok()).unwrap_or(0) as i32,
        is_active: crate::routes::admin_util::checkbox(&f.is_active),
    }
}

pub async fn create(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<BannerForm>,
) -> AppResult<Response> {
    db::banners::create(&s.db, &parse(&f)).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/banners").into_response())
}

pub async fn update(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<BannerForm>,
) -> AppResult<Response> {
    db::banners::update(&s.db, id, &parse(&f)).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/banners").into_response())
}

pub async fn delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    db::banners::delete(&s.db, id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/banners").into_response())
}

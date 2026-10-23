//! روت‌های مدیریت دسته‌بندی و برند (پنل ادمین).
//! هر تغییر کشِ صفحه‌ی اصلی را باطل می‌کند.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::{checkbox, clean_slug, parse_digits, parse_uuid_opt};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use db::catalog_admin::{self as cadmin, BrandInput, CategoryInput};
use db::models::{Brand, Category};
use serde::Deserialize;
use uuid::Uuid;

// ============================================================
//  دسته‌بندی
// ============================================================

#[derive(Template)]
#[template(path = "admin/categories.html")]
struct CategoriesTemplate {
    site_name: String,
    categories: Vec<cadmin::AdminCategoryRow>,
}

pub async fn categories_list(
    State(s): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Html<String>> {
    let categories = cadmin::list_categories_admin(&s.db).await?;
    render(CategoriesTemplate {
        site_name: s.site_name(),
        categories,
    })
}

#[derive(Template)]
#[template(path = "admin/category_form.html")]
struct CategoryFormTemplate {
    site_name: String,
    is_edit: bool,
    id: String,
    name: String,
    slug: String,
    parent_id: String,
    description: String,
    icon: String,
    sort_order: String,
    is_active: bool,
    // دسته‌های قابل‌انتخاب به‌عنوان والد (خودِ دسته در حالت ویرایش حذف می‌شود).
    parent_options: Vec<Category>,
    error: Option<String>,
}

/// گزینه‌های والد: همه‌ی دسته‌ها جز خودِ دسته‌ی در حال ویرایش (جلوگیری از حلقه).
async fn parent_options(s: &AppState, exclude: Option<Uuid>) -> AppResult<Vec<Category>> {
    let all = cadmin::all_categories(&s.db).await?;
    Ok(all.into_iter().filter(|c| Some(c.id) != exclude).collect())
}

pub async fn category_new(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let parent_options = parent_options(&s, None).await?;
    render(CategoryFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        parent_id: String::new(),
        description: String::new(),
        icon: String::new(),
        sort_order: "0".into(),
        is_active: true,
        parent_options,
        error: None,
    })
}

pub async fn category_edit(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let cat = cadmin::find_category(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("دسته".into()))?;
    render(build_category_template(&s, cat, None).await?)
}

async fn build_category_template(
    s: &AppState,
    c: Category,
    error: Option<String>,
) -> AppResult<CategoryFormTemplate> {
    let parent_options = parent_options(s, Some(c.id)).await?;
    Ok(CategoryFormTemplate {
        site_name: s.site_name(),
        is_edit: true,
        id: c.id.to_string(),
        name: c.name,
        slug: c.slug,
        parent_id: c.parent_id.map(|x| x.to_string()).unwrap_or_default(),
        description: c.description.unwrap_or_default(),
        icon: c.icon.unwrap_or_default(),
        sort_order: c.sort_order.to_string(),
        is_active: c.is_active,
        parent_options,
        error,
    })
}

#[derive(Deserialize)]
pub struct CategoryForm {
    pub name: String,
    pub slug: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<String>,
    pub is_active: Option<String>,
}

/// تجزیه‌ی فرم به CategoryInput. `self_id` برای جلوگیری از والد شدنِ خود.
fn parse_category_form(f: &CategoryForm, self_id: Option<Uuid>) -> Result<CategoryInput, String> {
    let name = f.name.trim().to_string();
    if name.is_empty() {
        return Err("نام دسته الزامی است.".into());
    }
    let slug = clean_slug(&f.slug);
    if slug.is_empty() {
        return Err("نامک (slug) باید حداقل یک حرف یا رقم انگلیسی داشته باشد.".into());
    }
    let parent_id = parse_uuid_opt(&f.parent_id);
    // دسته نمی‌تواند والد خودش باشد.
    if let (Some(p), Some(me)) = (parent_id, self_id) {
        if p == me {
            return Err("یک دسته نمی‌تواند والد خودش باشد.".into());
        }
    }
    let sort_order = f
        .sort_order
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_digits)
        .transpose()?
        .unwrap_or(0) as i32;

    Ok(CategoryInput {
        name,
        slug,
        parent_id,
        description: f
            .description
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        icon: f
            .icon
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        sort_order,
        is_active: checkbox(&f.is_active),
    })
}

pub async fn category_create(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<CategoryForm>,
) -> AppResult<Response> {
    let input = match parse_category_form(&f, None) {
        Ok(i) => i,
        Err(msg) => return category_form_error_new(&s, msg).await,
    };
    if cadmin::category_slug_exists(&s.db, &input.slug, None).await? {
        return category_form_error_new(&s, format!("نامک «{}» قبلاً استفاده شده است.", input.slug))
            .await;
    }
    cadmin::create_category(&s.db, &input).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/categories").into_response())
}

pub async fn category_update(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<CategoryForm>,
) -> AppResult<Response> {
    let Some(cat) = cadmin::find_category(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "دسته".into(),
        )));
    };
    let input = match parse_category_form(&f, Some(id)) {
        Ok(i) => i,
        Err(msg) => {
            return Ok(render(build_category_template(&s, cat, Some(msg)).await?)?.into_response())
        }
    };
    if cadmin::category_slug_exists(&s.db, &input.slug, Some(id)).await? {
        let msg = format!("نامک «{}» قبلاً استفاده شده است.", input.slug);
        return Ok(render(build_category_template(&s, cat, Some(msg)).await?)?.into_response());
    }
    cadmin::update_category(&s.db, id, &input).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/categories").into_response())
}

pub async fn category_delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    cadmin::delete_category(&s.db, id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/categories").into_response())
}

#[derive(Deserialize)]
pub struct ToggleForm {
    pub value: Option<String>,
}

pub async fn category_toggle_active(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<ToggleForm>,
) -> AppResult<Response> {
    cadmin::set_category_active(&s.db, id, checkbox(&f.value)).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/categories").into_response())
}

async fn category_form_error_new(s: &AppState, msg: String) -> AppResult<Response> {
    let parent_options = parent_options(s, None).await?;
    Ok(render(CategoryFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        parent_id: String::new(),
        description: String::new(),
        icon: String::new(),
        sort_order: "0".into(),
        is_active: true,
        parent_options,
        error: Some(msg),
    })?
    .into_response())
}

// ============================================================
//  برند
// ============================================================

#[derive(Template)]
#[template(path = "admin/brands.html")]
struct BrandsTemplate {
    site_name: String,
    brands: Vec<cadmin::AdminBrandRow>,
}

pub async fn brands_list(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let brands = cadmin::list_brands_admin(&s.db).await?;
    render(BrandsTemplate {
        site_name: s.site_name(),
        brands,
    })
}

#[derive(Template)]
#[template(path = "admin/brand_form.html")]
struct BrandFormTemplate {
    site_name: String,
    is_edit: bool,
    id: String,
    name: String,
    slug: String,
    logo: String,
    error: Option<String>,
}

pub async fn brand_new(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    render(BrandFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        logo: String::new(),
        error: None,
    })
}

pub async fn brand_edit(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let b = cadmin::find_brand(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("برند".into()))?;
    render(brand_template(&s, b, None))
}

fn brand_template(s: &AppState, b: Brand, error: Option<String>) -> BrandFormTemplate {
    BrandFormTemplate {
        site_name: s.site_name(),
        is_edit: true,
        id: b.id.to_string(),
        name: b.name,
        slug: b.slug,
        logo: b.logo.unwrap_or_default(),
        error,
    }
}

#[derive(Deserialize)]
pub struct BrandForm {
    pub name: String,
    pub slug: String,
    pub logo: Option<String>,
}

fn parse_brand_form(f: &BrandForm) -> Result<BrandInput, String> {
    let name = f.name.trim().to_string();
    if name.is_empty() {
        return Err("نام برند الزامی است.".into());
    }
    let slug = clean_slug(&f.slug);
    if slug.is_empty() {
        return Err("نامک (slug) باید حداقل یک حرف یا رقم انگلیسی داشته باشد.".into());
    }
    Ok(BrandInput {
        name,
        slug,
        logo: f
            .logo
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    })
}

pub async fn brand_create(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<BrandForm>,
) -> AppResult<Response> {
    let input = match parse_brand_form(&f) {
        Ok(i) => i,
        Err(msg) => return Ok(render(brand_error_new(&s, msg))?.into_response()),
    };
    if cadmin::brand_slug_exists(&s.db, &input.slug, None).await? {
        return Ok(render(brand_error_new(
            &s,
            format!("نامک «{}» قبلاً استفاده شده است.", input.slug),
        ))?
        .into_response());
    }
    cadmin::create_brand(&s.db, &input).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/brands").into_response())
}

pub async fn brand_update(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<BrandForm>,
) -> AppResult<Response> {
    let Some(b) = cadmin::find_brand(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "برند".into(),
        )));
    };
    let input = match parse_brand_form(&f) {
        Ok(i) => i,
        Err(msg) => return Ok(render(brand_template(&s, b, Some(msg)))?.into_response()),
    };
    if cadmin::brand_slug_exists(&s.db, &input.slug, Some(id)).await? {
        let msg = format!("نامک «{}» قبلاً استفاده شده است.", input.slug);
        return Ok(render(brand_template(&s, b, Some(msg)))?.into_response());
    }
    cadmin::update_brand(&s.db, id, &input).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/brands").into_response())
}

pub async fn brand_delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    cadmin::delete_brand(&s.db, id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/brands").into_response())
}

fn brand_error_new(s: &AppState, msg: String) -> BrandFormTemplate {
    BrandFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        logo: String::new(),
        error: Some(msg),
    }
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

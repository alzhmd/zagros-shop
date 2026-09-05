//! روت‌های مدیریت محصول (پنل ادمین): فهرست، ساخت، ویرایش، حذف،
//! و مدیریت تنوع‌ها و تصاویر. هر تغییر کشِ صفحه‌ی اصلی را باطل می‌کند.

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
use db::catalog_admin::{self as cadmin, ProductInput, VariantInput};
use db::models::{Brand, Category, Product, ProductImage, ProductVariant};
use serde::Deserialize;
use uuid::Uuid;

// ===================== فهرست محصولات =====================

#[derive(Template)]
#[template(path = "admin/products.html")]
struct ProductsTemplate {
    site_name: String,
    products: Vec<cadmin::AdminProductRow>,
    search: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
}

pub async fn list(
    State(s): State<AppState>,
    _admin: AdminUser,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> AppResult<Html<String>> {
    let products = cadmin::list_all_products(&s.db, q.q.as_deref()).await?;
    render(ProductsTemplate {
        site_name: s.site_name(),
        products,
        search: q.q.unwrap_or_default(),
    })
}

// ===================== فرم ساخت/ویرایش =====================

#[derive(Template)]
#[template(path = "admin/product_form.html")]
struct ProductFormTemplate {
    site_name: String,
    is_edit: bool,
    // مقادیر فعلی (در حالت ساخت، خالی).
    id: String,
    name: String,
    slug: String,
    category_id: String,
    brand_id: String,
    short_desc: String,
    description: String,
    price_toman: String,
    is_active: bool,
    is_featured: bool,
    video_url: String,
    deal_ends: String,
    categories: Vec<Category>,
    brands: Vec<Brand>,
    variants: Vec<ProductVariant>,
    images: Vec<ProductImage>,
    attrs: Vec<db::attributes::ProductAttrRow>,
    all_attrs: Vec<db::attributes::Attribute>,
    error: Option<String>,
}

/// صفحه‌ی ساخت محصول جدید (فرم خالی).
pub async fn new_form(State(s): State<AppState>, _admin: AdminUser) -> AppResult<Html<String>> {
    let categories = cadmin::all_categories(&s.db).await?;
    let brands = cadmin::all_brands(&s.db).await?;
    let all_attrs = db::attributes::all_attributes(&s.db).await?;
    render(ProductFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        category_id: String::new(),
        brand_id: String::new(),
        short_desc: String::new(),
        description: String::new(),
        price_toman: String::new(),
        is_active: true,
        is_featured: false,
        video_url: String::new(),
        deal_ends: String::new(),
        categories,
        brands,
        variants: Vec::new(),
        images: Vec::new(),
        attrs: Vec::new(),
        all_attrs,
        error: None,
    })
}

/// صفحه‌ی ویرایش یک محصول موجود (با تنوع‌ها و تصاویر).
pub async fn edit_form(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let product = cadmin::find_product(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("محصول".into()))?;
    render(build_edit_template(&s, product, None).await?)
}

/// قالب ویرایش را از روی یک محصول می‌سازد (برای نمایش و نمایش خطا).
async fn build_edit_template(
    s: &AppState,
    product: Product,
    error: Option<String>,
) -> AppResult<ProductFormTemplate> {
    let categories = cadmin::all_categories(&s.db).await?;
    let brands = cadmin::all_brands(&s.db).await?;
    let variants = cadmin::product_variants(&s.db, product.id).await?;
    let images = cadmin::product_images(&s.db, product.id).await?;
    let attrs = db::attributes::product_attr_rows(&s.db, product.id).await?;
    let all_attrs = db::attributes::all_attributes(&s.db).await?;
    Ok(ProductFormTemplate {
        site_name: s.site_name(),
        is_edit: true,
        id: product.id.to_string(),
        name: product.name,
        slug: product.slug,
        category_id: product.category_id.map(|x| x.to_string()).unwrap_or_default(),
        brand_id: product.brand_id.map(|x| x.to_string()).unwrap_or_default(),
        short_desc: product.short_desc.unwrap_or_default(),
        description: product.description.unwrap_or_default(),
        price_toman: domain::Rial(product.base_price).to_toman().to_string(),
        is_active: product.is_active,
        is_featured: product.is_featured,
        video_url: product.video_url.unwrap_or_default(),
        // datetime-local نیازمند فرمت «YYYY-MM-DDTHH:MM» به وقت ایران است (UTC + ۳:۳۰).
        deal_ends: product
            .deal_ends_at
            .map(|t| (t + chrono::Duration::minutes(IRAN_OFFSET_MIN)).format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default(),
        categories,
        brands,
        variants,
        images,
        attrs,
        all_attrs,
        error,
    })
}

// ===================== ساخت/به‌روزرسانی =====================

#[derive(Deserialize)]
pub struct ProductForm {
    pub name: String,
    pub slug: String,
    pub category_id: Option<String>,
    pub brand_id: Option<String>,
    pub short_desc: Option<String>,
    pub description: Option<String>,
    pub price_toman: Option<String>,
    pub is_active: Option<String>,
    pub is_featured: Option<String>,
    pub video_url: Option<String>,
    pub deal_ends: Option<String>,
}

/// تجزیه‌ی ورودی فرم به ProductInput (مشترک بین ساخت و ویرایش).
fn parse_product_form(f: &ProductForm) -> Result<ProductInput, String> {
    let name = f.name.trim().to_string();
    if name.is_empty() {
        return Err("نام محصول الزامی است.".into());
    }
    let slug = clean_slug(&f.slug);
    if slug.is_empty() {
        return Err("نامک (slug) باید حداقل یک حرف یا رقم انگلیسی داشته باشد.".into());
    }
    // قیمت تومان → ریال.
    let toman: i64 = f
        .price_toman
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_digits)
        .transpose()?
        .unwrap_or(0);
    if toman < 0 {
        return Err("قیمت نامعتبر است.".into());
    }

    Ok(ProductInput {
        name,
        slug,
        category_id: parse_uuid_opt(&f.category_id),
        brand_id: parse_uuid_opt(&f.brand_id),
        short_desc: f.short_desc.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        description: f.description.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        base_price: domain::Rial::from_toman(toman).0,
        is_active: checkbox(&f.is_active),
        is_featured: checkbox(&f.is_featured),
        video_url: f.video_url.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        deal_ends_at: parse_deal_ends(f.deal_ends.as_deref()),
    })
}

/// آفست منطقه‌زمانی ایران از UTC: +۳:۳۰ (بدون ساعت تابستانی از ۲۰۲۲).
const IRAN_OFFSET_MIN: i64 = 210;

/// تجزیه‌ی datetime-local («YYYY-MM-DDTHH:MM») که ادمین به وقت ایران وارد می‌کند، به UTC.
fn parse_deal_ends(s: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.map(str::trim).filter(|s| !s.is_empty())?;
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .ok()
        .map(|naive| naive.and_utc() - chrono::Duration::minutes(IRAN_OFFSET_MIN))
}

pub async fn create(
    State(s): State<AppState>,
    _admin: AdminUser,
    Form(f): Form<ProductForm>,
) -> AppResult<Response> {
    let input = match parse_product_form(&f) {
        Ok(i) => i,
        Err(msg) => return form_error_new(&s, msg).await,
    };
    if cadmin::slug_exists(&s.db, &input.slug, None).await? {
        return form_error_new(&s, format!("نامک «{}» قبلاً استفاده شده است.", input.slug)).await;
    }
    let product = cadmin::create_product(&s.db, &input).await?;
    s.home_cache.invalidate();
    // پس از ساخت، به صفحه‌ی ویرایش می‌رویم تا تنوع/تصویر اضافه شود.
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product.id)).into_response())
}

pub async fn update(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<ProductForm>,
) -> AppResult<Response> {
    let Some(product) = cadmin::find_product(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("محصول".into())));
    };
    let input = match parse_product_form(&f) {
        Ok(i) => i,
        Err(msg) => return Ok(render(build_edit_template(&s, product, Some(msg)).await?)?.into_response()),
    };
    if cadmin::slug_exists(&s.db, &input.slug, Some(id)).await? {
        let msg = format!("نامک «{}» قبلاً استفاده شده است.", input.slug);
        return Ok(render(build_edit_template(&s, product, Some(msg)).await?)?.into_response());
    }
    cadmin::update_product(&s.db, id, &input).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", id)).into_response())
}

pub async fn delete(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    cadmin::delete_product(&s.db, id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/products").into_response())
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
    cadmin::set_active(&s.db, id, checkbox(&f.value)).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/products").into_response())
}

pub async fn toggle_featured(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<ToggleForm>,
) -> AppResult<Response> {
    cadmin::set_featured(&s.db, id, checkbox(&f.value)).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to("/admin/products").into_response())
}

// ===================== تنوع‌ها =====================

#[derive(Deserialize)]
pub struct VariantForm {
    pub title: String,
    pub sku: Option<String>,
    pub price_toman: String,
    pub compare_toman: Option<String>,
    pub stock: Option<String>,
    pub sort_order: Option<String>,
    pub is_active: Option<String>,
}

fn parse_variant_form(f: &VariantForm) -> Result<VariantInput, String> {
    let title = f.title.trim().to_string();
    if title.is_empty() {
        return Err("عنوان تنوع الزامی است.".into());
    }
    let toman = parse_digits(f.price_toman.trim())?;
    // قیمت قبل از تخفیف (اختیاری) — فقط اگر بزرگ‌تر از قیمت فروش باشد معنادار است.
    let compare_at_price = f
        .compare_toman
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_digits)
        .transpose()?
        .map(|t| domain::Rial::from_toman(t).0)
        .filter(|c| *c > domain::Rial::from_toman(toman).0);
    let stock = f
        .stock
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_digits)
        .transpose()?
        .unwrap_or(0) as i32;
    let sort_order = f
        .sort_order
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_digits)
        .transpose()?
        .unwrap_or(0) as i32;
    Ok(VariantInput {
        title,
        sku: f.sku.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        price: domain::Rial::from_toman(toman).0,
        compare_at_price,
        stock,
        is_active: checkbox(&f.is_active),
        sort_order,
    })
}

pub async fn add_variant(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(product_id): Path<Uuid>,
    Form(f): Form<VariantForm>,
) -> AppResult<Response> {
    let v = parse_variant_form(&f)
        .map_err(|m| AppError::Domain(domain::DomainError::Invalid(m)))?;
    cadmin::add_variant(&s.db, product_id, &v).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

pub async fn update_variant(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path((product_id, variant_id)): Path<(Uuid, Uuid)>,
    Form(f): Form<VariantForm>,
) -> AppResult<Response> {
    let v = parse_variant_form(&f)
        .map_err(|m| AppError::Domain(domain::DomainError::Invalid(m)))?;
    // وضعیت قبل از تغییر (برای تشخیص موجودشدن/کاهش قیمت → اعلان).
    let before = cadmin::variant_snapshot(&s.db, variant_id).await.ok().flatten();
    cadmin::update_variant(&s.db, variant_id, &v).await?;
    if let Some((old_price, old_stock, pid, pname, pslug)) = before {
        // موجود شد (۰ → مثبت): مشترکانِ هشدار را خبر کن.
        if old_stock <= 0 && v.stock > 0 {
            let _ = db::notifications::notify_back_in_stock(&s.db, variant_id, &pname, &pslug).await;
        }
        // کاهش قیمت: علاقه‌مندان را خبر کن.
        if v.price < old_price {
            let _ = db::notifications::notify_price_drop(&s.db, pid, &pname, &pslug).await;
        }
    }
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

pub async fn delete_variant(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path((product_id, variant_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Response> {
    cadmin::delete_variant(&s.db, variant_id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

// ===================== تصاویر =====================

/// افزودن تصویر: آپلود فایل (multipart) یا واردکردن مسیر/URL دستی.
pub async fn add_image(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(product_id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> AppResult<Response> {
    let mut uploaded: Option<String> = None;
    let mut manual_path: Option<String> = None;
    let mut alt: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Domain(domain::DomainError::Invalid(format!("آپلود نامعتبر: {e}"))))?
    {
        match field.name() {
            Some("file") => {
                match crate::upload::save_image_field(field).await {
                    Ok(Some(p)) => uploaded = Some(p),
                    Ok(None) => {}
                    Err(msg) => return Err(AppError::Domain(domain::DomainError::Invalid(msg))),
                }
            }
            Some("path") => {
                let v = field.text().await.unwrap_or_default();
                let v = v.trim().to_string();
                if !v.is_empty() {
                    manual_path = Some(v);
                }
            }
            Some("alt") => {
                let v = field.text().await.unwrap_or_default();
                let v = v.trim().to_string();
                if !v.is_empty() {
                    alt = Some(v);
                }
            }
            _ => {}
        }
    }

    let path = uploaded.or(manual_path).ok_or_else(|| {
        AppError::Domain(domain::DomainError::Invalid(
            "یک فایل تصویر انتخاب کنید یا مسیر تصویر را وارد کنید.".into(),
        ))
    })?;
    cadmin::add_image(&s.db, product_id, &path, alt.as_deref()).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

pub async fn delete_image(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path((product_id, image_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Response> {
    cadmin::delete_image(&s.db, image_id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

pub async fn set_primary_image(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path((product_id, image_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Response> {
    cadmin::set_primary_image(&s.db, image_id).await?;
    s.home_cache.invalidate();
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

// ===================== ویژگی‌ها (مشخصات فنی) =====================

#[derive(Deserialize)]
pub struct AttrForm {
    pub attribute_id: Uuid,
    pub value: Option<String>,
}

pub async fn set_attr(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(product_id): Path<Uuid>,
    Form(f): Form<AttrForm>,
) -> AppResult<Response> {
    let value = f.value.unwrap_or_default();
    db::attributes::set_value(&s.db, product_id, f.attribute_id, &value).await?;
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

#[derive(Deserialize)]
pub struct NewAttrForm {
    pub name: String,
}

/// ساخت یک ویژگی جدید (سراسری) — مثلاً «گارانتی».
pub async fn create_attr(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(product_id): Path<Uuid>,
    Form(f): Form<NewAttrForm>,
) -> AppResult<Response> {
    let name = f.name.trim();
    if !name.is_empty() {
        let slug = crate::routes::admin_util::clean_slug(name);
        let slug = if slug.is_empty() {
            format!("attr-{}", &Uuid::new_v4().to_string()[..8])
        } else {
            slug
        };
        db::attributes::create_attribute(&s.db, name, &slug).await?;
    }
    Ok(Redirect::to(&format!("/admin/products/{}/edit", product_id)).into_response())
}

// ===================== کمک‌توابع =====================

async fn form_error_new(s: &AppState, msg: String) -> AppResult<Response> {
    let categories = cadmin::all_categories(&s.db).await?;
    let brands = cadmin::all_brands(&s.db).await?;
    let all_attrs = db::attributes::all_attributes(&s.db).await?;
    Ok(render(ProductFormTemplate {
        site_name: s.site_name(),
        is_edit: false,
        id: String::new(),
        name: String::new(),
        slug: String::new(),
        category_id: String::new(),
        brand_id: String::new(),
        short_desc: String::new(),
        description: String::new(),
        price_toman: String::new(),
        is_active: true,
        is_featured: false,
        video_url: String::new(),
        deal_ends: String::new(),
        categories,
        brands,
        variants: Vec::new(),
        images: Vec::new(),
        attrs: Vec::new(),
        all_attrs,
        error: Some(msg),
    })?
    .into_response())
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

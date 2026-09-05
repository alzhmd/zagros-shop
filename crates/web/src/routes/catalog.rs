use crate::{error::AppResult, state::AppState};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use db::catalog::{self, ProductQuery, ProductSort};
use db::models::{Brand, Category, ProductCard, Product, ProductImage, ProductVariant};
use serde::{Deserialize, Deserializer};

use crate::filters::{toman_string, to_fa_digits};

#[derive(Debug, Deserialize, Default)]
pub struct CatalogParams {
    pub cat: Option<String>,
    pub brand: Option<String>,
    pub q: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none_i64")]
    pub min: Option<i64>,
    #[serde(default, deserialize_with = "empty_as_none_i64")]
    pub max: Option<i64>,
    pub sort: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none_i64")]
    pub page: Option<i64>,
    /// فقط موجود.
    pub stock: Option<String>,
    /// فقط تخفیف‌دار.
    pub off: Option<String>,
    /// حداقل امتیاز.
    #[serde(default, deserialize_with = "empty_as_none_i64")]
    pub rating: Option<i64>,
}

/// تبدیل مقدار query به Option<i64>؛ رشته‌ی خالی یا نامعتبر → None
/// (فرم کاتالوگ همیشه min= و max= خالی هم می‌فرستد؛ نباید کل صفحه ۴۰۰ شود).
fn empty_as_none_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i64>().ok()))
}

/// دسته‌بندی والد به‌همراه زیردسته‌ها برای نمایش درختی.
struct CategoryNode {
    slug: String,
    name: String,
    children: Vec<Category>,
}

#[derive(Template)]
#[template(path = "catalog.html")]
struct CatalogTemplate {
    site_name: String,
    canonical: String,
    parent_categories: Vec<CategoryNode>,
    brands: Vec<Brand>,
    products: Vec<ProductCard>,
    result_label: String,
    total_fa: String,
    total_pages: i64,
    has_prev: bool,
    has_next: bool,
    prev_url: String,
    next_url: String,
    sort_base: String,
    active_sort: String,
    active_cat: String,
    active_brand: String,
    search_value: String,
    min_value: String,
    max_value: String,
    stock_on: bool,
    off_on: bool,
    rating_val: i64,
    attr_groups: Vec<AttrFilterGroup>,
    page_links: Vec<PageLink>,
    active_chips: Vec<ActiveChip>,
}

struct PageLink {
    num_fa: String,
    url: String,
    current: bool,
}

/// یک چیپ فیلتر فعال با لینک حذف آن فیلتر.
struct ActiveChip {
    label: String,
    remove_url: String,
}

/// یک ویژگی قابل‌فیلتر با مقادیرش و وضعیت انتخاب (برای نوار فیلتر).
struct AttrFilterGroup {
    slug: String,
    name: String,
    values: Vec<AttrFilterValue>,
}
struct AttrFilterValue {
    value: String,
    selected: bool,
}

fn build_tree(categories: Vec<Category>) -> Vec<CategoryNode> {
    let parents: Vec<&Category> = categories.iter().filter(|c| c.parent_id.is_none()).collect();
    parents
        .iter()
        .map(|p| CategoryNode {
            slug: p.slug.clone(),
            name: p.name.clone(),
            children: categories
                .iter()
                .filter(|c| c.parent_id == Some(p.id))
                .cloned()
                .collect(),
        })
        .collect()
}

/// ساخت query string برای صفحه‌بندی با حفظ فیلترهای فعلی.
fn page_url(p: &CatalogParams, page: i64) -> String {
    let mut parts = vec![format!("page={page}")];
    if let Some(c) = &p.cat {
        parts.push(format!("cat={c}"));
    }
    if let Some(b) = &p.brand {
        parts.push(format!("brand={b}"));
    }
    if let Some(q) = &p.q {
        parts.push(format!("q={q}"));
    }
    if let Some(m) = p.min {
        parts.push(format!("min={m}"));
    }
    if let Some(m) = p.max {
        parts.push(format!("max={m}"));
    }
    if let Some(s) = &p.sort {
        parts.push(format!("sort={s}"));
    }
    if p.stock.as_deref() == Some("1") {
        parts.push("stock=1".into());
    }
    if p.off.as_deref() == Some("1") {
        parts.push("off=1".into());
    }
    if let Some(r) = p.rating {
        parts.push(format!("rating={r}"));
    }
    format!("/products?{}", parts.join("&"))
}

/// پایه‌ی URL برای تغییر مرتب‌سازی (همه‌ی فیلترها جز sort).
fn sort_base(p: &CatalogParams) -> String {
    let mut parts = vec![];
    if let Some(c) = &p.cat {
        parts.push(format!("cat={c}"));
    }
    if let Some(b) = &p.brand {
        parts.push(format!("brand={b}"));
    }
    if let Some(q) = &p.q {
        parts.push(format!("q={q}"));
    }
    if let Some(m) = p.min {
        parts.push(format!("min={m}"));
    }
    if let Some(m) = p.max {
        parts.push(format!("max={m}"));
    }
    if p.stock.as_deref() == Some("1") {
        parts.push("stock=1".into());
    }
    if p.off.as_deref() == Some("1") {
        parts.push("off=1".into());
    }
    if let Some(r) = p.rating {
        parts.push(format!("rating={r}"));
    }
    if parts.is_empty() {
        "/products?".to_string()
    } else {
        format!("/products?{}&", parts.join("&"))
    }
}

pub async fn index(
    State(state): State<AppState>,
    Query(params): Query<CatalogParams>,
    Query(all_pairs): Query<Vec<(String, String)>>,
) -> AppResult<Html<String>> {
    // استخراج فیلترهای ویژگی از پارامترهای `attr.<slug>=<value>` (چندمقداری مجاز).
    let mut attr_map: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for (k, v) in &all_pairs {
        if let Some(slug) = k.strip_prefix("attr.") {
            if !slug.is_empty() && !v.trim().is_empty() {
                attr_map.entry(slug.to_string()).or_default().push(v.clone());
            }
        }
    }
    let attrs_vec: Vec<(String, Vec<String>)> = attr_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    // پسوند query برای حفظ فیلتر ویژگی‌ها در لینک‌های صفحه‌بندی/مرتب‌سازی.
    let attr_qs: String = attr_map
        .iter()
        .flat_map(|(slug, vals)| vals.iter().map(move |v| format!("&attr.{}={}", slug, urlenc(v))))
        .collect();

    let sort = ProductSort::parse(params.sort.as_deref().unwrap_or("newest"));
    let in_stock_only = params.stock.as_deref() == Some("1");
    let discounted_only = params.off.as_deref() == Some("1");
    let min_rating = params.rating.unwrap_or(0).clamp(0, 5);
    let q = ProductQuery {
        category_slug: params.cat.clone(),
        brand_slug: params.brand.clone(),
        search: params.q.clone(),
        min_price: params.min.map(|t| t * 10),
        max_price: params.max.map(|t| t * 10),
        attrs: attrs_vec,
        in_stock_only,
        discounted_only,
        min_rating,
        sort,
        page: params.page.unwrap_or(1),
        per_page: 12,
    };

    let categories = catalog::list_categories(&state.db).await?;
    let brands = catalog::list_brands(&state.db).await?;
    let paged = catalog::list_products(&state.db, &q).await?;

    // ویژگی‌های قابل‌فیلتر با وضعیت انتخاب.
    let filterable = db::attributes::filterable(&state.db).await.unwrap_or_default();
    let attr_groups: Vec<AttrFilterGroup> = filterable
        .into_iter()
        .map(|fa| {
            let selected_vals = attr_map.get(&fa.slug);
            AttrFilterGroup {
                values: fa
                    .values
                    .into_iter()
                    .map(|v| AttrFilterValue {
                        selected: selected_vals.map(|s| s.contains(&v)).unwrap_or(false),
                        value: v,
                    })
                    .collect(),
                slug: fa.slug,
                name: fa.name,
            }
        })
        .collect();

    let result_label = match &params.q {
        Some(s) if !s.trim().is_empty() => format!("نتایج جستجو برای «{}»", s),
        _ => "همه‌ی محصولات".to_string(),
    };

    // canonical: تثبیت روی بُعد اصلی (دسته یا «همه») تا نسخه‌های مرتب‌سازی/صفحه‌بندی/جستجو یکی شوند.
    let base_url = state.base_url.to_string();
    let canonical = match &params.cat {
        Some(c) if !c.is_empty() => format!("{base_url}/products?cat={c}"),
        _ => format!("{base_url}/products"),
    };

    // شماره‌ی صفحه‌ها (پنجره‌ی حداکثر ۷ تایی حول صفحه‌ی فعلی).
    let total_pages = paged.total_pages();
    let cur = paged.page;
    let (lo, hi) = {
        let mut lo = (cur - 3).max(1);
        let mut hi = (lo + 6).min(total_pages);
        lo = (hi - 6).max(1);
        if hi < lo {
            hi = lo;
        }
        (lo, hi)
    };
    let page_links: Vec<PageLink> = (lo..=hi)
        .map(|n| PageLink {
            num_fa: to_fa_digits(&n.to_string()),
            url: format!("{}{}", page_url(&params, n), attr_qs),
            current: n == cur,
        })
        .collect();

    // چیپ‌های فیلتر فعال (با لینک حذف هر فیلتر).
    let mut active_chips: Vec<ActiveChip> = Vec::new();
    if let Some(qv) = params.q.as_deref().filter(|s| !s.trim().is_empty()) {
        active_chips.push(ActiveChip {
            label: format!("جستجو: {qv}"),
            remove_url: filters_without(&params, &attr_map, Some("q"), None),
        });
    }
    if let Some(bv) = params.brand.as_deref().filter(|s| !s.is_empty()) {
        let name = brands.iter().find(|b| b.slug == bv).map(|b| b.name.clone()).unwrap_or_else(|| bv.to_string());
        active_chips.push(ActiveChip {
            label: format!("برند: {name}"),
            remove_url: filters_without(&params, &attr_map, Some("brand"), None),
        });
    }
    for (slug, vals) in &attr_map {
        for v in vals {
            active_chips.push(ActiveChip {
                label: v.clone(),
                remove_url: filters_without(&params, &attr_map, Some("attr"), Some((slug, v))),
            });
        }
    }

    let tpl = CatalogTemplate {
        site_name: state.site_name(),
        canonical,
        parent_categories: build_tree(categories),
        brands,
        result_label,
        total_fa: to_fa_digits(&paged.total.to_string()),
        total_pages: paged.total_pages(),
        has_prev: paged.has_prev(),
        has_next: paged.has_next(),
        prev_url: format!("{}{}", page_url(&params, paged.page - 1), attr_qs),
        next_url: format!("{}{}", page_url(&params, paged.page + 1), attr_qs),
        sort_base: format!("{}{}", sort_base(&params), attr_qs),
        active_sort: sort.as_str().to_string(),
        active_cat: params.cat.clone().unwrap_or_default(),
        active_brand: params.brand.clone().unwrap_or_default(),
        search_value: params.q.clone().unwrap_or_default(),
        min_value: params.min.map(|v| v.to_string()).unwrap_or_default(),
        max_value: params.max.map(|v| v.to_string()).unwrap_or_default(),
        stock_on: in_stock_only,
        off_on: discounted_only,
        rating_val: min_rating,
        products: paged.items,
        attr_groups,
        page_links,
        active_chips,
    };
    render(&tpl)
}

/// بازسازی URL فیلترها با حذف یک فیلتر مشخص (برای چیپ‌های قابل‌حذف).
/// `drop_kind`: "q" | "brand" | "attr". برای attr، `drop_attr` جفت (slug,value) حذف‌شدنی است.
fn filters_without(
    p: &CatalogParams,
    attr_map: &std::collections::BTreeMap<String, Vec<String>>,
    drop_kind: Option<&str>,
    drop_attr: Option<(&String, &String)>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = p.cat.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("cat={}", urlenc(c)));
    }
    if drop_kind != Some("brand") {
        if let Some(b) = p.brand.as_deref().filter(|s| !s.is_empty()) {
            parts.push(format!("brand={}", urlenc(b)));
        }
    }
    if drop_kind != Some("q") {
        if let Some(q) = p.q.as_deref().filter(|s| !s.trim().is_empty()) {
            parts.push(format!("q={}", urlenc(q)));
        }
    }
    if let Some(m) = p.min {
        parts.push(format!("min={m}"));
    }
    if let Some(m) = p.max {
        parts.push(format!("max={m}"));
    }
    for (slug, vals) in attr_map {
        for v in vals {
            // اگر این همان جفت حذف‌شدنی است، نگذار.
            if drop_kind == Some("attr") {
                if let Some((ds, dv)) = drop_attr {
                    if ds == slug && dv == v {
                        continue;
                    }
                }
            }
            parts.push(format!("attr.{}={}", slug, urlenc(v)));
        }
    }
    if parts.is_empty() {
        "/products".to_string()
    } else {
        format!("/products?{}", parts.join("&"))
    }
}

#[derive(Template)]
#[template(path = "product.html")]
struct ProductTemplate {
    site_name: String,
    base_url: String,
    canonical: String,
    og_image: Option<String>,
    p: Product,
    brand: Option<Brand>,
    variants: Vec<ProductVariant>,
    images: Vec<ProductImage>,
    min_price_fa: String,
    offer_price: i64,
    cheapest_has_discount: bool,
    cheapest_compare_fa: String,
    cheapest_pct_fa: String,
    deal_ends_iso: Option<String>,
    in_stock: bool,
    breadcrumb_cat: Option<(String, String)>,
    in_wishlist: bool,
    product_id: String,
    rating: db::reviews::RatingSummary,
    reviews: Vec<db::reviews::Review>,
    attrs: Vec<db::attributes::AttrValue>,
    related: Vec<ProductCard>,
    logged_in: bool,
    review_author: String,
    video_embed: Option<String>,
    recent: Vec<ProductCard>,
    questions: Vec<(db::qa::Question, Vec<db::qa::Answer>)>,
}

const RECENT_COOKIE: &str = "zagros_recent";

pub async fn product(
    State(state): State<AppState>,
    crate::extract::CurrentUser(user): crate::extract::CurrentUser,
    jar: axum_extra::extract::CookieJar,
    Path(slug): Path<String>,
) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;
    let detail = catalog::product_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("محصول".into()))?;

    let db = state.db.clone();
    let pid = detail.product.id;
    tokio::spawn(async move {
        let _ = catalog::increment_view(&db, pid).await;
    });

    // ارزان‌ترین تنوع (برای قیمت و تخفیف نمایش اولیه‌ی صفحه).
    let cheapest = detail.variants.iter().min_by_key(|v| v.price);
    let min_price = cheapest
        .map(|v| v.price)
        .unwrap_or(detail.product.base_price);
    let cheapest_has_discount = cheapest.map(|v| v.has_discount()).unwrap_or(false);
    let cheapest_compare_fa = cheapest.map(|v| v.compare_at_fa()).unwrap_or_default();
    let cheapest_pct_fa = cheapest.map(|v| v.discount_pct_fa()).unwrap_or_default();
    // تایمر پیشنهاد لحظه‌ای (در صورت فعال‌بودن و نگذشتن مهلت).
    let deal_ends_iso = detail
        .product
        .deal_ends_at
        .filter(|t| *t > chrono::Utc::now())
        .map(|t| t.to_rfc3339());
    let in_stock = detail.variants.iter().any(|v| v.stock > 0);
    let breadcrumb_cat = detail
        .category
        .as_ref()
        .map(|c| (c.slug.clone(), c.name.clone()));

    let base_url = state.base_url.to_string();
    let canonical = format!("{}/product/{}", base_url, detail.product.slug);
    let og_image = detail
        .images
        .first()
        .map(|img| absolute_url(&base_url, &img.path));

    // داده‌های نقد/امتیاز/ویژگی/علاقه‌مندی/مرتبط.
    let rating = db::reviews::summary(&state.db, pid).await.unwrap_or_default();
    let reviews = db::reviews::approved_for_product(&state.db, pid).await.unwrap_or_default();
    let attrs = db::attributes::for_product(&state.db, pid).await.unwrap_or_default();
    let related = catalog::related_products(&state.db, pid, detail.product.category_id, 4)
        .await
        .unwrap_or_default();
    let questions = db::qa::approved_for_product(&state.db, pid).await.unwrap_or_default();

    let (in_wishlist, review_author) = match &user {
        Some(u) => (
            db::wishlist::contains(&state.db, u.id, pid).await.unwrap_or(false),
            u.full_name.clone().unwrap_or_default(),
        ),
        None => (false, String::new()),
    };
    let video_embed = detail.product.video_url.as_deref().and_then(embed_url);

    // بازدیدهای اخیر: شناسه‌ها از کوکی (به‌جز محصول جاری)، سپس کوکی به‌روزرسانی می‌شود.
    let prev_ids: Vec<uuid::Uuid> = jar
        .get(RECENT_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| uuid::Uuid::parse_str(s.trim()).ok())
        .filter(|id| *id != pid)
        .take(8)
        .collect();
    let recent = catalog::cards_by_ids(&state.db, &prev_ids).await.unwrap_or_default();
    // فهرست جدید: محصول جاری در ابتدا، سپس قبلی‌ها، یکتا، حداکثر ۸.
    let mut new_ids: Vec<String> = vec![pid.to_string()];
    for id in &prev_ids {
        let s = id.to_string();
        if !new_ids.contains(&s) {
            new_ids.push(s);
        }
    }
    new_ids.truncate(8);
    let mut cookie = axum_extra::extract::cookie::Cookie::new(RECENT_COOKIE, new_ids.join(","));
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::days(30));
    let jar = jar.add(cookie);

    let tpl = ProductTemplate {
        site_name: state.site_name(),
        base_url,
        canonical,
        og_image,
        brand: detail.brand,
        variants: detail.variants,
        images: detail.images,
        min_price_fa: toman_string(min_price),
        offer_price: min_price,
        cheapest_has_discount,
        cheapest_compare_fa,
        cheapest_pct_fa,
        deal_ends_iso,
        in_stock,
        breadcrumb_cat,
        in_wishlist,
        product_id: pid.to_string(),
        rating,
        reviews,
        attrs,
        related,
        logged_in: user.is_some(),
        review_author,
        video_embed,
        recent,
        questions,
        p: detail.product,
    };
    let html = render(&tpl)?;
    Ok((jar, html).into_response())
}

/// تبدیل لینک یوتیوب/آپارات به URL امبد (در صورت پشتیبانی)؛ وگرنه None.
fn embed_url(url: &str) -> Option<String> {
    let u = url.trim();
    if u.is_empty() {
        return None;
    }
    if let Some(id) = u.split("watch?v=").nth(1) {
        let id = id.split('&').next().unwrap_or(id);
        return Some(format!("https://www.youtube.com/embed/{id}"));
    }
    if u.contains("youtu.be/") {
        if let Some(id) = u.rsplit('/').next() {
            return Some(format!("https://www.youtube.com/embed/{id}"));
        }
    }
    if u.contains("/embed/") || u.contains("aparat.com/video/video/embed") {
        return Some(u.to_string());
    }
    None
}

/// صفحه‌ی برند: به کاتالوگ فیلترشده با برند هدایت می‌کند (URL تمیز برای SEO).
pub async fn brand_landing(Path(slug): Path<String>) -> axum::response::Redirect {
    axum::response::Redirect::to(&format!("/products?brand={}", urlenc(&slug)))
}

// ===================== صفحه‌ی دسته‌بندی‌ها =====================

#[derive(Template)]
#[template(path = "categories.html")]
struct CategoriesTemplate {
    site_name: String,
    canonical: String,
    parents: Vec<CategoryNode>,
    brands: Vec<Brand>,
}

/// صفحه‌ی مرور دسته‌بندی‌ها (درختی) + برندها.
pub async fn categories_page(State(state): State<AppState>) -> AppResult<Html<String>> {
    let categories = catalog::list_categories(&state.db).await?;
    let brands = catalog::list_brands(&state.db).await?;
    render(&CategoriesTemplate {
        site_name: state.site_name(),
        canonical: format!("{}/categories", state.base_url),
        parents: build_tree(categories),
        brands,
    })
}

/// مسیر تصویر را به URL مطلق تبدیل می‌کند (اگر از قبل مطلق نباشد).
fn absolute_url(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if let Some(rest) = path.strip_prefix('/') {
        format!("{base}/{rest}")
    } else {
        format!("{base}/{path}")
    }
}

fn render<T: Template>(t: &T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))
}

/// کدگذاری درصدی ساده‌ی مقدار query (برای مقادیر فارسی ویژگی‌ها در لینک‌ها).
fn urlenc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

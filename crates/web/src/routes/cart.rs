use crate::auth::CART_COOKIE;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar};
use db::cart::CartItemView;
use serde::Deserialize;
use uuid::Uuid;

/// گرفتن کلید سبد از کوکی یا ساخت کوکی جدید.
pub fn ensure_cart_cookie(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    if let Some(c) = jar.get(CART_COOKIE) {
        let key = c.value().to_string();
        (jar, key)
    } else {
        let key = Uuid::new_v4().to_string();
        let mut cookie = Cookie::new(CART_COOKIE, key.clone());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_max_age(time::Duration::days(30));
        (jar.add(cookie), key)
    }
}

#[derive(Template)]
#[template(path = "cart.html")]
struct CartTemplate {
    site_name: String,
    items: Vec<CartItemView>,
    saved: Vec<CartItemView>,
    total_fa: String,
    is_empty: bool,
    suggested: Vec<db::models::ProductCard>,
    recent: Vec<db::models::ProductCard>,
}

#[derive(Template)]
#[template(path = "partials/cart_body.html")]
struct CartBodyTemplate {
    items: Vec<CartItemView>,
    saved: Vec<CartItemView>,
    total_fa: String,
    is_empty: bool,
}

/// پاسخ HTMX برای تغییر سبد: فقط بدنه‌ی سبد را برمی‌گرداند (به‌جای ریدایرکت).
async fn cart_body_response(s: &AppState, cart_id: Uuid) -> AppResult<axum::response::Response> {
    let items = db::cart::items(&s.db, cart_id).await?;
    let saved = db::cart::saved_items(&s.db, cart_id)
        .await
        .unwrap_or_default();
    let total: i64 = items.iter().map(|i| i.line_total()).sum();
    let body = CartBodyTemplate {
        is_empty: items.is_empty() && saved.is_empty(),
        total_fa: domain::Rial(total).format_toman_fa(),
        items,
        saved,
    }
    .render()
    .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    let mut resp = Html(body).into_response();
    resp.headers_mut().insert(
        "HX-Trigger",
        axum::http::HeaderValue::from_static("cart-changed"),
    );
    Ok(resp)
}

pub async fn view(
    State(s): State<AppState>,
    plain: axum_extra::extract::CookieJar,
    jar: SignedCookieJar,
) -> AppResult<axum::response::Response> {
    let (jar, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    let items = db::cart::items(&s.db, cart_id).await?;
    let saved = db::cart::saved_items(&s.db, cart_id)
        .await
        .unwrap_or_default();
    let total: i64 = items.iter().map(|i| i.line_total()).sum();

    // پیشنهاد مکمل (پرفروش‌ها) + بازدیدهای اخیر (از کوکی).
    let suggested = if items.is_empty() {
        Vec::new()
    } else {
        db::catalog::most_viewed(&s.db, 4).await.unwrap_or_default()
    };
    let recent_ids: Vec<uuid::Uuid> = plain
        .get("zagros_recent")
        .map(|c| c.value().to_string())
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| uuid::Uuid::parse_str(s.trim()).ok())
        .take(4)
        .collect();
    let recent = db::catalog::cards_by_ids(&s.db, &recent_ids)
        .await
        .unwrap_or_default();

    let tpl = CartTemplate {
        site_name: s.site_name(),
        is_empty: items.is_empty() && saved.is_empty(),
        total_fa: domain::Rial(total).format_toman_fa(),
        items,
        saved,
        suggested,
        recent,
    };
    let html = tpl
        .render()
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    Ok((jar, Html(html)).into_response())
}

/// ذخیره برای بعد / بازگرداندن به سبد.
pub async fn save_for_later(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<UpdateItemForm>,
) -> AppResult<axum::response::Response> {
    let (_, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    db::cart::set_saved(&s.db, cart_id, form.item_id, true).await?;
    cart_body_response(&s, cart_id).await
}

pub async fn move_to_cart(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<UpdateItemForm>,
) -> AppResult<axum::response::Response> {
    let (_, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    db::cart::set_saved(&s.db, cart_id, form.item_id, false).await?;
    cart_body_response(&s, cart_id).await
}

#[derive(Deserialize)]
pub struct UpdateItemForm {
    pub item_id: Uuid,
}

#[derive(Deserialize)]
pub struct AddForm {
    pub variant_id: Uuid,
    #[serde(default = "one")]
    pub qty: i32,
    /// «buy» = خرید سریع (مستقیم به تسویه).
    #[serde(default)]
    pub action: Option<String>,
}
fn one() -> i32 {
    1
}

pub async fn add(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<AddForm>,
) -> AppResult<axum::response::Response> {
    let (jar, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    let qty = form.qty.max(1);
    db::cart::add_item(&s.db, cart_id, form.variant_id, qty).await?;

    let buy_now = form.action.as_deref() == Some("buy");

    // درخواست HTMX (افزودن از صفحه‌ی محصول): بدون ترک صفحه، فقط رویداد به‌روزرسانی هدر + toast.
    // خرید سریع: با HX-Redirect مستقیم به تسویه می‌رویم.
    if crate::web::is_htmx(&headers) {
        let mut resp = (axum::http::StatusCode::OK, jar, "افزوده شد").into_response();
        if buy_now {
            resp.headers_mut().insert(
                "HX-Redirect",
                axum::http::HeaderValue::from_static("/checkout"),
            );
        } else {
            resp.headers_mut().insert(
                "HX-Trigger",
                axum::http::HeaderValue::from_static("cart-changed"),
            );
        }
        return Ok(resp);
    }
    let dest = if buy_now { "/checkout" } else { "/cart" };
    Ok((jar, Redirect::to(dest)).into_response())
}

#[derive(Deserialize)]
pub struct UpdateForm {
    pub item_id: Uuid,
    pub qty: i32,
}

pub async fn update(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<UpdateForm>,
) -> AppResult<axum::response::Response> {
    let (jar, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    db::cart::set_qty(&s.db, cart_id, form.item_id, form.qty).await?;
    if crate::web::is_htmx(&headers) {
        return cart_body_response(&s, cart_id).await;
    }
    Ok((jar, Redirect::to("/cart")).into_response())
}

#[derive(Deserialize)]
pub struct RemoveForm {
    pub item_id: Uuid,
}

pub async fn remove(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<RemoveForm>,
) -> AppResult<axum::response::Response> {
    let (jar, key) = ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    db::cart::remove_item(&s.db, cart_id, form.item_id).await?;
    if crate::web::is_htmx(&headers) {
        return cart_body_response(&s, cart_id).await;
    }
    Ok((jar, Redirect::to("/cart")).into_response())
}

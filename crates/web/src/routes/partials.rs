//! پاسخ‌های جزئی (HTMX): چرومِ هدر (سبد + ورود)، فوتر، و پیشنهاد جستجو.
//! این‌ها در همه‌ی صفحات یک‌بار با HTMX بارگذاری می‌شوند تا state سراسری
//! (تعداد سبد، وضعیت ورود) بدون تزریق در همه‌ی قالب‌ها در دسترس باشد.

use crate::auth::CART_COOKIE;
use crate::extract::CurrentUser;
use crate::state::AppState;
use crate::web::render;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_extra::extract::cookie::SignedCookieJar;
use db::models::ProductCard;
use serde::Deserialize;

use crate::error::AppResult;

#[derive(Template)]
#[template(path = "partials/header.html")]
struct HeaderPartial {
    user_name: Option<String>,
    is_admin: bool,
    unread_fa: Option<String>,
}

/// چرومِ هدر: وضعیت ورود (نام کاربر/مدیریت). سبد جدا در cart_mini است.
pub async fn header(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let user_name = user.as_ref().map(|u| {
        u.full_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| u.mobile.clone())
    });
    let is_admin = user.as_ref().map(|u| u.role == "admin").unwrap_or(false);
    // شمار اعلان‌های خوانده‌نشده (فقط برای کاربر واردشده).
    let unread_fa = match &user {
        Some(u) => {
            let n = db::notifications::unread_count(&s.db, u.id)
                .await
                .unwrap_or(0);
            if n > 0 {
                Some(crate::filters::to_fa_digits(&n.to_string()))
            } else {
                None
            }
        }
        None => None,
    };
    render(HeaderPartial {
        user_name,
        is_admin,
        unread_fa,
    })
}

#[derive(Template)]
#[template(path = "partials/menu.html")]
struct MenuPartial {
    user_name: Option<String>,
    is_admin: bool,
}

/// لینک‌های حساب/مدیریت داخل منوی کشویی موبایل.
pub async fn menu(
    State(_s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let user_name = user.as_ref().map(|u| {
        u.full_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| u.mobile.clone())
    });
    let is_admin = user.as_ref().map(|u| u.role == "admin").unwrap_or(false);
    render(MenuPartial {
        user_name,
        is_admin,
    })
}

#[derive(Template)]
#[template(path = "partials/cart_mini.html")]
struct CartMini {
    cart_count: i64,
    cart_count_fa: String,
}

/// نشانگر سبد (آیکن داخل لینک سبد در هدر).
pub async fn cart_mini(State(s): State<AppState>, jar: SignedCookieJar) -> AppResult<Html<String>> {
    let cart_count = match jar.get(CART_COOKIE).map(|c| c.value().to_string()) {
        Some(key) => match db::cart::find_cart(&s.db, &key).await? {
            Some(cart_id) => db::cart::item_count(&s.db, cart_id).await?,
            None => 0,
        },
        None => 0,
    };
    render(CartMini {
        cart_count,
        cart_count_fa: crate::filters::to_fa_digits(&cart_count.to_string()),
    })
}

/// فقط عددِ تعداد سبد (برای نشان روی تب‌بار موبایل) — خالی وقتی صفر است.
pub async fn cart_mini_badge(
    State(s): State<AppState>,
    jar: SignedCookieJar,
) -> AppResult<Html<String>> {
    let cart_count = match jar.get(CART_COOKIE).map(|c| c.value().to_string()) {
        Some(key) => match db::cart::find_cart(&s.db, &key).await? {
            Some(cart_id) => db::cart::item_count(&s.db, cart_id).await?,
            None => 0,
        },
        None => 0,
    };
    let body = if cart_count > 0 {
        crate::filters::to_fa_digits(&cart_count.to_string())
    } else {
        String::new()
    };
    Ok(Html(body))
}

#[derive(Template)]
#[template(path = "partials/footer.html")]
struct FooterPartial {
    contact_address: String,
    contact_phone: String,
}

/// فوترِ تماس از تنظیمات زنده.
pub async fn footer(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(FooterPartial {
        contact_address: s.settings.contact_address(),
        contact_phone: s.settings.contact_phone(),
    })
}

#[derive(Template)]
#[template(path = "partials/search_suggest.html")]
struct SearchSuggest {
    products: Vec<ProductCard>,
    q: String,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

/// پیشنهاد زنده‌ی جستجو (autocomplete) — حداکثر ۶ نتیجه.
pub async fn search(
    State(s): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Html<String>> {
    let term = q.q.unwrap_or_default();
    let trimmed = term.trim();
    if trimmed.chars().count() < 2 {
        return Ok(Html(String::new()));
    }
    let products = db::catalog::search_suggest(&s.db, trimmed, 6).await?;
    render(SearchSuggest {
        products,
        q: trimmed.to_string(),
    })
}

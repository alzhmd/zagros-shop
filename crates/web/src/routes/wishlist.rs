//! علاقه‌مندی‌ها (Wishlist) — فاز ارتقا.

use crate::error::AppResult;
use crate::extract::CurrentUser;
use crate::state::AppState;
use crate::web::{is_htmx, render};
use askama::Template;
use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use db::models::ProductCard;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ToggleForm {
    pub product_id: Uuid,
}

#[derive(Template)]
#[template(path = "partials/wishlist_button.html")]
struct WishlistButton {
    product_id: Uuid,
    in_wishlist: bool,
}

/// افزودن/حذف از علاقه‌مندی‌ها. نیاز به ورود. پاسخ HTMX = دکمه‌ی به‌روزشده.
pub async fn toggle(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
    Form(f): Form<ToggleForm>,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let now_in = db::wishlist::toggle(&s.db, user.id, f.product_id).await?;
    if is_htmx(&headers) {
        return Ok(render(WishlistButton {
            product_id: f.product_id,
            in_wishlist: now_in,
        })?
        .into_response());
    }
    Ok(Redirect::to("/account/wishlist").into_response())
}

#[derive(Template)]
#[template(path = "account/wishlist.html")]
struct WishlistPage {
    site_name: String,
    products: Vec<ProductCard>,
    points_fa: String,
    user: db::accounts::User,
}

pub async fn page(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let products = db::wishlist::list_cards(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    Ok(render(WishlistPage {
        site_name: s.site_name(),
        products,
        points_fa,
        user,
    })?
    .into_response())
}

//! صفحه‌ی پیشنهادها (پیشنهادهای ویژه + پرفروش‌ها + جدیدترین‌ها) — فاز ارتقا.

use crate::{error::AppResult, state::AppState, web::render};
use askama::Template;
use axum::{extract::State, response::Html};
use db::models::ProductCard;

#[derive(Template)]
#[template(path = "offers.html")]
struct OffersTemplate {
    site_name: String,
    canonical: String,
    featured: Vec<ProductCard>,
    popular: Vec<ProductCard>,
    newest: Vec<ProductCard>,
}

pub async fn index(State(s): State<AppState>) -> AppResult<Html<String>> {
    let featured = db::catalog::featured_products(&s.db, 8)
        .await
        .unwrap_or_default();
    let popular = db::catalog::most_viewed(&s.db, 8).await.unwrap_or_default();
    let newest = db::catalog::newest_products(&s.db, 8)
        .await
        .unwrap_or_default();
    render(OffersTemplate {
        site_name: s.site_name(),
        canonical: format!("{}/offers", s.base_url),
        featured,
        popular,
        newest,
    })
}

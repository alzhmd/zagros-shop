use crate::{error::AppResult, state::AppState};
use askama::Template;
use axum::extract::State;
use axum::response::Html;
use db::banners::Banner;
use db::catalog;
use db::models::{Category, ProductCard};

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    site_name: String,
    base_url: String,
    banners: Vec<Banner>,
    strip: Option<Banner>,
    featured: Vec<ProductCard>,
    deals: Vec<ProductCard>,
    popular: Vec<ProductCard>,
    newest: Vec<ProductCard>,
    categories: Vec<Category>,
}

pub async fn index(State(state): State<AppState>) -> AppResult<Html<String>> {
    // مسیر داغ: از کش درون‌حافظه‌ای می‌خوانیم؛ در صورت نبود، از دیتابیس و سپس کش می‌کنیم.
    let data = match state.home_cache.get() {
        Some(d) => d,
        None => {
            let banners = db::banners::active_for(&state.db, "hero").await.unwrap_or_default();
            let strip = db::banners::active_for(&state.db, "strip")
                .await
                .unwrap_or_default()
                .into_iter()
                .next();
            let featured = catalog::featured_products(&state.db, 8).await?;
            let deals = catalog::deal_products(&state.db, 8).await.unwrap_or_default();
            let popular = catalog::most_viewed(&state.db, 8).await.unwrap_or_default();
            let newest = catalog::newest_products(&state.db, 8).await.unwrap_or_default();
            let categories = catalog::list_categories(&state.db).await?;
            let top_categories: Vec<Category> =
                categories.into_iter().filter(|c| c.parent_id.is_none()).collect();
            let d = crate::state::HomeData {
                banners,
                strip,
                featured,
                deals,
                popular,
                newest,
                categories: top_categories,
            };
            state.home_cache.set(d.clone());
            d
        }
    };

    let tpl = HomeTemplate {
        site_name: state.site_name(),
        base_url: state.base_url.to_string(),
        banners: data.banners,
        strip: data.strip,
        featured: data.featured,
        deals: data.deals,
        popular: data.popular,
        newest: data.newest,
        categories: data.categories,
    };
    tpl.render()
        .map(Html)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))
}

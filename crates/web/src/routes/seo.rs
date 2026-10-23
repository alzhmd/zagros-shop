use crate::{error::AppResult, state::AppState};
use axum::{extract::State, http::header, response::IntoResponse};
use db::catalog;

/// robots.txt — اجازه‌ی ایندکس کامل + اشاره به sitemap با URL مطلق.
pub async fn robots(State(state): State<AppState>) -> impl IntoResponse {
    let body = format!(
        "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n",
        state.base_url.as_ref()
    );
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

/// sitemap.xml پویا — صفحات ثابت + همه‌ی محصولات (با lastmod) و دسته‌ها.
pub async fn sitemap(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let base = state.base_url.as_ref();

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );

    // صفحات ثابت.
    for path in [
        "/",
        "/products",
        "/categories",
        "/offers",
        "/repair",
        "/support",
        "/about",
        "/contact",
        "/faq",
        "/terms",
    ] {
        xml.push_str(&format!(
            "  <url><loc>{}</loc></url>\n",
            escape_xml(&format!("{base}{path}"))
        ));
    }

    // دسته‌ها
    for c in catalog::list_categories(&state.db).await? {
        xml.push_str(&format!(
            "  <url><loc>{}</loc></url>\n",
            escape_xml(&format!("{base}/products?cat={}", c.slug))
        ));
    }

    // محصولات با تاریخ آخرین تغییر (lastmod، فرمت W3C).
    for (slug, updated_at) in catalog::active_slugs_with_lastmod(&state.db).await? {
        let loc = escape_xml(&format!("{base}/product/{slug}"));
        let lastmod = updated_at.format("%Y-%m-%d").to_string();
        xml.push_str(&format!(
            "  <url><loc>{loc}</loc><lastmod>{lastmod}</lastmod></url>\n"
        ));
    }

    xml.push_str("</urlset>\n");

    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    ))
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

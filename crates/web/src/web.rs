//! کمک‌توابع مشترک لایه‌ی وب — رندر قالب و پاسخ‌های HTMX.

use crate::error::{AppError, AppResult};
use askama::Template;
use axum::response::Html;

/// رندر یک قالب askama به پاسخ HTML (جایگزین مشترک ده‌ها تابع `render` تکراری).
pub fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

/// آیا این درخواست از HTMX آمده؟ (برای پاسخ جزئی به‌جای صفحه‌ی کامل.)
pub fn is_htmx(headers: &axum::http::HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

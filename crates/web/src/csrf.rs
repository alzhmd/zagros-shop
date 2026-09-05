//! محافظت CSRF با بررسی Origin/Referer روی درخواست‌های تغییردهنده (POST/PUT/PATCH/DELETE).
//!
//! رویکرد: چون همه‌ی کوکی‌های نشست/سبد با `SameSite=Lax` ست شده‌اند، مرورگر اصلاً
//! کوکی را در POSTِ بین‌سایتی نمی‌فرستد. این میان‌افزار یک لایه‌ی دفاعی دوم است:
//! هدر `Origin` (یا در نبودش `Referer`) باید با میزبان درخواست یکی باشد. روش
//! بدون توکن است؛ نیازی به تغییر ده‌ها فرم/HTMX نیست و با same-origin سازگار است.

use axum::{
    body::Body,
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// آیا متد، تغییردهنده است؟ (متدهای امن نیاز به بررسی ندارند.)
fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// میزبان مجاز را از هدر `Host` می‌گیرد و با میزبانِ `Origin`/`Referer` مقایسه می‌کند.
pub async fn guard(req: Request<Body>, next: Next) -> Response {
    if !is_mutating(req.method()) {
        return next.run(req).await;
    }

    let headers = req.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // میزبانِ منبع از Origin (ترجیحی) یا Referer.
    let source = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .filter(|s| *s != "null")
        .or_else(|| headers.get(header::REFERER).and_then(|v| v.to_str().ok()))
        .and_then(host_of);

    match (host, source) {
        // هر دو موجود و یکی → مجاز.
        (Some(h), Some(src)) if hosts_match(&h, &src) => next.run(req).await,
        // نه Origin و نه Referer (مثلاً برخی کلاینت‌های غیرمرورگر): چون SameSite=Lax
        // از CSRF کوکی‌محور جلوگیری می‌کند، اجازه می‌دهیم تا چیزی نشکند.
        (_, None) => next.run(req).await,
        // ناسازگار → رد.
        _ => (
            StatusCode::FORBIDDEN,
            "درخواست نامعتبر (CSRF). صفحه را تازه کنید و دوباره تلاش کنید.",
        )
            .into_response(),
    }
}

/// استخراج بخش host:port از یک URL مطلق (origin یا referer).
fn host_of(url: &str) -> Option<String> {
    // الگو: scheme://host[:port]/...
    let after_scheme = url.split("://").nth(1)?;
    let host = after_scheme.split('/').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// مقایسه‌ی میزبان‌ها؛ پورت پیش‌فرض را نادیده می‌گیریم تا پروکسی/TLS مشکل نسازد.
fn hosts_match(a: &str, b: &str) -> bool {
    strip_default_port(a) == strip_default_port(b)
}

fn strip_default_port(h: &str) -> &str {
    h.strip_suffix(":80")
        .or_else(|| h.strip_suffix(":443"))
        .unwrap_or(h)
}

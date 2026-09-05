use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use domain::DomainError;

/// خطای سطح وب — هر خطای دامنه/زیرساخت را به پاسخ HTTP مناسب نگاشت می‌کند.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

/// صفحه‌ی خطای کامل (مستقل از base تا حتی هنگام خرابی state هم رندر شود).
#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage {
    site_name: &'static str,
    code: u16,
    title: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, title, message) = match &self {
            AppError::Domain(DomainError::NotFound(_)) => (
                StatusCode::NOT_FOUND,
                "صفحه یافت نشد",
                "صفحه یا کالایی که دنبالش بودید پیدا نشد.".to_string(),
            ),
            AppError::Domain(DomainError::Invalid(m)) => {
                (StatusCode::BAD_REQUEST, "درخواست نامعتبر", m.clone())
            }
            AppError::Domain(DomainError::Unauthorized) => (
                StatusCode::UNAUTHORIZED,
                "دسترسی غیرمجاز",
                "برای دیدن این صفحه باید وارد شوید یا دسترسی لازم را ندارید.".to_string(),
            ),
            AppError::Domain(DomainError::Conflict(m)) => {
                (StatusCode::CONFLICT, "تعارض", m.clone())
            }
            AppError::Domain(DomainError::OutOfStock(m)) => (
                StatusCode::CONFLICT,
                "موجودی کافی نیست",
                format!("موجودی کافی نیست: {m}"),
            ),
            _ => {
                tracing::error!(error = ?self, "خطای داخلی سرور");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "خطای داخلی سرور",
                    "مشکلی پیش آمد. لطفاً کمی بعد دوباره تلاش کنید.".to_string(),
                )
            }
        };

        let page = ErrorPage {
            site_name: "زاگرس — جهان کولاک",
            code: status.as_u16(),
            title,
            message,
        };
        match page.render() {
            Ok(html) => (status, Html(html)).into_response(),
            Err(_) => (status, title).into_response(),
        }
    }
}

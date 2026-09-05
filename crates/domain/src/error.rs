use thiserror::Error;

pub type DomainResult<T> = Result<T, DomainError>;

/// خطاهای سطح دامنه — مستقل از لایه‌ی انتقال (HTTP) و ذخیره‌سازی.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("یافت نشد: {0}")]
    NotFound(String),

    #[error("ورودی نامعتبر: {0}")]
    Invalid(String),

    #[error("دسترسی غیرمجاز")]
    Unauthorized,

    #[error("تعارض: {0}")]
    Conflict(String),

    #[error("موجودی کافی نیست برای «{0}»")]
    OutOfStock(String),
}

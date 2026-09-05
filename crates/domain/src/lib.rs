//! منطق دامنه‌ی کسب‌وکار زاگرس — مستقل از وب و دیتابیس.

pub mod error;
pub mod money;

pub use error::{DomainError, DomainResult};
pub use money::{fa_digits, Rial};

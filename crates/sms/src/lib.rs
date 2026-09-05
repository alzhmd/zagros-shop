//! انتزاع ارسال پیامک OTP — با آداپتورهای کاوه‌نگار، SMS.ir و کنسول (توسعه).

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SmsError {
    #[error("خطای شبکه: {0}")]
    Network(String),
    #[error("خطای ارائه‌دهنده ({code}): {message}")]
    Provider { code: i64, message: String },
}

pub struct SmsReceipt {
    pub message_id: String,
}

/// رابط مشترک ارسال کد یکبارمصرف. کد در اپ تولید/منقضی می‌شود؛
/// آداپتور فقط آن را در قالب از پیش‌تأییدشده تزریق می‌کند.
#[async_trait]
pub trait SmsSender: Send + Sync {
    async fn send_otp(&self, mobile: &str, code: &str) -> Result<SmsReceipt, SmsError>;
    fn name(&self) -> &'static str;
}

mod console;
mod kavenegar;
mod smsir;

pub use console::ConsoleSms;
pub use kavenegar::Kavenegar;
pub use smsir::SmsIr;

/// ساخت فرستنده بر اساس نام ارائه‌دهنده‌ی فعال در تنظیمات.
pub fn build(provider: &str, api_key: &str, template: &str) -> Box<dyn SmsSender> {
    match provider {
        "kavenegar" => Box::new(Kavenegar::new(api_key, template)),
        "smsir" => Box::new(SmsIr::new(api_key, template)),
        _ => Box::new(ConsoleSms),
    }
}

//! انتزاع درگاه پرداخت ایرانی — زرین‌پال و زیبال پشت یک رابط مشترک.
//! همه‌ی مبالغ داخلی به **ریال** هستند.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GwError {
    #[error("خطای شبکه: {0}")]
    Network(String),
    #[error("خطای درگاه ({code}): {message}")]
    Gateway { code: i64, message: String },
    #[error("مبلغ نامعتبر")]
    InvalidAmount,
}

/// درخواست شروع پرداخت.
#[derive(Debug, Clone)]
pub struct PayRequest {
    pub amount_rial: u64,
    pub order_id: String,
    pub callback_url: String,
    pub description: String,
    pub mobile: Option<String>,
}

/// نتیجه‌ی شروع: توکن درگاه + آدرس ریدایرکت.
#[derive(Debug, Clone)]
pub struct PayInit {
    pub gateway_token: String,
    pub redirect_url: String,
}

/// نتیجه‌ی تأیید پرداخت.
#[derive(Debug, Clone)]
pub struct PayResult {
    pub paid: bool,
    pub already_verified: bool,
    pub ref_id: String,
    pub card_pan: Option<String>,
}

/// رابط مشترک هر دو درگاه: request → redirect → verify.
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    fn name(&self) -> &'static str;
    async fn request(&self, r: &PayRequest) -> Result<PayInit, GwError>;
    /// `token` و `amount` از callback می‌آیند؛ همیشه مبلغ دوباره بررسی می‌شود.
    async fn verify(&self, token: &str, amount_rial: u64) -> Result<PayResult, GwError>;
}

mod zarinpal;
mod zibal;

pub use zarinpal::Zarinpal;
pub use zibal::Zibal;

/// ساخت درگاه فعال بر اساس تنظیمات.
pub fn build(provider: &str, merchant: &str) -> Option<Box<dyn PaymentGateway>> {
    match provider {
        "zarinpal" => Some(Box::new(Zarinpal::new(merchant))),
        "zibal" => Some(Box::new(Zibal::new(merchant))),
        _ => None,
    }
}

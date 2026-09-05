use crate::{SmsError, SmsReceipt, SmsSender};
use async_trait::async_trait;

/// آداپتور توسعه: کد را در لاگ چاپ می‌کند (بدون ارسال واقعی).
/// زمانی که پنل پیامک هنوز خریداری نشده.
pub struct ConsoleSms;

#[async_trait]
impl SmsSender for ConsoleSms {
    async fn send_otp(&self, mobile: &str, code: &str) -> Result<SmsReceipt, SmsError> {
        tracing::info!(mobile = %mobile, code = %code, "📱 [ConsoleSms] کد OTP (حالت توسعه)");
        Ok(SmsReceipt {
            message_id: "console".to_string(),
        })
    }
    fn name(&self) -> &'static str {
        "console"
    }
}

//! ارسال ایمیل (lettre، خودمیزبان). اگر SMTP پیکربندی نشده باشد، فقط لاگ می‌شود
//! (حالت توسعه/کنسول) — دقیقاً مثل آداپتور پیامک، تا بدون سرویس خارجی هم اجرا شود.

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

#[derive(Clone)]
pub struct Mailer {
    inner: Arc<MailerInner>,
}

struct MailerInner {
    from: String,
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl Mailer {
    pub fn new(host: &str, port: u16, user: &str, pass: &str, from: &str) -> Self {
        let transport = if host.trim().is_empty() {
            None
        } else {
            let builder = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map(|b| b.port(port))
                .map(|b| {
                    if !user.is_empty() {
                        b.credentials(Credentials::new(user.to_string(), pass.to_string()))
                    } else {
                        b
                    }
                });
            match builder {
                Ok(b) => Some(b.build()),
                Err(e) => {
                    tracing::error!(error = %e, "پیکربندی SMTP شکست خورد؛ ایمیل در حالت کنسول");
                    None
                }
            }
        };
        Self {
            inner: Arc::new(MailerInner {
                from: from.to_string(),
                transport,
            }),
        }
    }

    /// ارسال یک ایمیل متنی. خطا فقط لاگ می‌شود (نباید جریان اصلی را بشکند).
    pub async fn send(&self, to: &str, subject: &str, body: &str) {
        let Some(transport) = &self.inner.transport else {
            tracing::info!(to, subject, "📧 [console mail] {body}");
            return;
        };
        let from = match self.inner.from.parse() {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(error = %e, "آدرس فرستنده‌ی ایمیل نامعتبر");
                return;
            }
        };
        let to_addr = match to.parse() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, to, "آدرس گیرنده‌ی ایمیل نامعتبر");
                return;
            }
        };
        let msg = Message::builder()
            .from(from)
            .to(to_addr)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string());
        match msg {
            Ok(m) => {
                if let Err(e) = transport.send(m).await {
                    tracing::error!(error = %e, "ارسال ایمیل شکست خورد");
                }
            }
            Err(e) => tracing::error!(error = %e, "ساخت پیام ایمیل شکست خورد"),
        }
    }
}

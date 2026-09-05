//! ربات تلگرام «زاگرس — جهان کولاک» (فاز ۵).
//! آداپتور سبک روی reqwest با long-polling؛ اتصال حساب، امتیاز، و اطلاع‌رسانی.
//! یک باینری، چند نقش: این ربات به‌صورت یک تسک tokio جدا از وب اجرا می‌شود.

mod api;
mod handlers;

pub use api::{BotError, TelegramApi};

use db::Db;
use std::sync::Arc;

/// هندل قابل‌اشتراک ربات برای ارسال اعلان از بخش‌های دیگر اپ (مثلاً پس از پرداخت).
#[derive(Clone)]
pub struct BotHandle {
    api: TelegramApi,
}

impl BotHandle {
    pub fn new(token: &str) -> Self {
        Self {
            api: TelegramApi::new(token),
        }
    }

    /// ارسال یک اعلان به کاربری که حسابش به تلگرام متصل است.
    /// خطا فقط لاگ می‌شود (اعلان نباید جریان اصلی را بشکند).
    pub async fn notify(&self, telegram_id: i64, text: &str) {
        if let Err(e) = self.api.send_message(telegram_id, text).await {
            tracing::warn!(error = %e, telegram_id, "ارسال اعلان تلگرام ناموفق");
        }
    }

    /// نام کاربری ربات (بدون @)؛ برای ساخت لینک t.me. در صورت خطا رشته‌ی خالی.
    pub async fn username(&self) -> String {
        self.api.get_me().await.unwrap_or_default()
    }
}

/// حلقه‌ی اصلی long-polling ربات. تا قطع‌شدن برنامه ادامه می‌دهد.
/// این تابع را با `tokio::spawn` در یک تسک جدا اجرا کنید.
pub async fn run(token: String, db: Db) {
    let api = TelegramApi::new(&token);

    // بررسی سلامت اتصال در راه‌اندازی.
    match api.get_me().await {
        Ok(username) => tracing::info!(bot = %username, "ربات تلگرام متصل شد"),
        Err(e) => {
            tracing::error!(error = %e, "اتصال اولیه‌ی ربات تلگرام شکست خورد؛ تلاش ادامه می‌یابد");
        }
    }

    let db = Arc::new(db);
    let mut offset: i64 = 0;
    const POLL_TIMEOUT: u32 = 30; // ثانیه (long-poll)

    loop {
        match api.get_updates(offset, POLL_TIMEOUT).await {
            Ok(updates) => {
                for update in updates {
                    offset = update.update_id + 1;
                    if let Some(msg) = update.message {
                        // chat_id برای پاسخ؛ from.id همان شناسه‌ی کاربر برای اتصال حساب.
                        let chat_id = msg.chat.id;
                        let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(chat_id);
                        if let Some(text) = msg.text {
                            let api = api.clone();
                            let db = Arc::clone(&db);
                            // پردازش هر پیام جدا تا یک خطا حلقه را کند نکند.
                            tokio::spawn(async move {
                                if let Some(reply) =
                                    handlers::handle_message(&db, user_id, &text).await
                                {
                                    if let Err(e) = api.send_message(chat_id, &reply).await {
                                        tracing::warn!(error = %e, "پاسخ ربات ارسال نشد");
                                    }
                                }
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // خطای شبکه/تلگرام: کمی صبر و تلاش دوباره (در برابر قطعی اینترنت مقاوم).
                tracing::warn!(error = %e, "getUpdates ناموفق؛ ۵ ثانیه صبر");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

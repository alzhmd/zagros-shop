//! کلاینت سبک Telegram Bot API روی reqwest (بدون فریم‌ورک).
//! فقط متدها و فیلدهای موردنیاز پیاده شده‌اند.

use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("خطای شبکه‌ی تلگرام: {0}")]
    Network(String),
    #[error("خطای API تلگرام: {0}")]
    Api(String),
}

/// کلاینت قابل‌اشتراک تلگرام. ارزان clone می‌شود (reqwest::Client داخلی Arc است).
#[derive(Clone)]
pub struct TelegramApi {
    base: String,
    client: reqwest::Client,
}

impl TelegramApi {
    pub fn new(token: &str) -> Self {
        Self {
            base: format!("https://api.telegram.org/bot{token}"),
            client: reqwest::Client::new(),
        }
    }

    /// فراخوانی یک متد API با بدنه‌ی JSON و مهلت مشخص؛ فیلد `result` را برمی‌گرداند.
    async fn call(
        &self,
        method: &str,
        body: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, BotError> {
        let url = format!("{}/{}", self.base, method);
        let resp = self
            .client
            .post(&url)
            .timeout(timeout)
            .json(&body)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        if json["ok"].as_bool() == Some(true) {
            Ok(json["result"].clone())
        } else {
            let desc = json["description"]
                .as_str()
                .unwrap_or("خطای نامشخص")
                .to_string();
            Err(BotError::Api(desc))
        }
    }

    /// نام کاربری ربات (برای بررسی سلامت اتصال در راه‌اندازی).
    pub async fn get_me(&self) -> Result<String, BotError> {
        let r = self
            .call("getMe", serde_json::json!({}), Duration::from_secs(15))
            .await?;
        Ok(r["username"].as_str().unwrap_or_default().to_string())
    }

    /// دریافت به‌روزرسانی‌ها با long-polling. مهلت درخواست = timeout + حاشیه.
    pub async fn get_updates(
        &self,
        offset: i64,
        timeout_secs: u32,
    ) -> Result<Vec<Update>, BotError> {
        let body = serde_json::json!({
            "offset": offset,
            "timeout": timeout_secs,
            "allowed_updates": ["message"],
        });
        let r = self
            .call(
                "getUpdates",
                body,
                Duration::from_secs(timeout_secs as u64 + 15),
            )
            .await?;
        let updates: Vec<Update> =
            serde_json::from_value(r).map_err(|e| BotError::Api(e.to_string()))?;
        Ok(updates)
    }

    /// ارسال پیام متنی ساده (بدون parse_mode → بدون نیاز به escape).
    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<(), BotError> {
        let body = serde_json::json!({ "chat_id": chat_id, "text": text });
        self.call("sendMessage", body, Duration::from_secs(15))
            .await?;
        Ok(())
    }
}

// ---------- انواع به‌روزرسانی (فقط فیلدهای لازم) ----------

#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    pub update_id: i64,
    #[serde(default)]
    pub message: Option<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub from: Option<TgUser>,
    pub chat: Chat,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TgUser {
    pub id: i64,
    #[serde(default)]
    pub first_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Chat {
    pub id: i64,
}

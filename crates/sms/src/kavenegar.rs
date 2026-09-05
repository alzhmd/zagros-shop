use crate::{SmsError, SmsReceipt, SmsSender};
use async_trait::async_trait;

/// آداپتور کاوه‌نگار — Verify Lookup API.
/// GET https://api.kavenegar.com/v1/{KEY}/verify/lookup.json?receptor=&token=&template=
pub struct Kavenegar {
    api_key: String,
    template: String,
    client: reqwest::Client,
}

impl Kavenegar {
    pub fn new(api_key: &str, template: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            template: template.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SmsSender for Kavenegar {
    async fn send_otp(&self, mobile: &str, code: &str) -> Result<SmsReceipt, SmsError> {
        let url = format!(
            "https://api.kavenegar.com/v1/{}/verify/lookup.json",
            self.api_key
        );
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("receptor", mobile),
                ("token", code),
                ("template", &self.template),
            ])
            .send()
            .await
            .map_err(|e| SmsError::Network(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SmsError::Network(e.to_string()))?;

        let status = json["return"]["status"].as_i64().unwrap_or(0);
        if status == 200 {
            let mid = json["entries"][0]["messageid"]
                .as_i64()
                .map(|x| x.to_string())
                .unwrap_or_default();
            Ok(SmsReceipt { message_id: mid })
        } else {
            let message = json["return"]["message"]
                .as_str()
                .unwrap_or("خطای نامشخص کاوه‌نگار")
                .to_string();
            Err(SmsError::Provider {
                code: status,
                message,
            })
        }
    }
    fn name(&self) -> &'static str {
        "kavenegar"
    }
}

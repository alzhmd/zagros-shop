use crate::{SmsError, SmsReceipt, SmsSender};
use async_trait::async_trait;
use serde_json::json;

/// آداپتور SMS.ir — Send Verify API.
/// POST https://api.sms.ir/v1/send/verify  (هدر x-api-key)
pub struct SmsIr {
    api_key: String,
    template_id: i64,
    client: reqwest::Client,
}

impl SmsIr {
    pub fn new(api_key: &str, template: &str) -> Self {
        // در SMS.ir شناسه‌ی قالب عدد است.
        let template_id = template.trim().parse::<i64>().unwrap_or(0);
        Self {
            api_key: api_key.to_string(),
            template_id,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SmsSender for SmsIr {
    async fn send_otp(&self, mobile: &str, code: &str) -> Result<SmsReceipt, SmsError> {
        let body = json!({
            "mobile": mobile,
            "templateId": self.template_id,
            "parameters": [ { "name": "OTP", "value": code } ]
        });
        let resp = self
            .client
            .post("https://api.sms.ir/v1/send/verify")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SmsError::Network(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SmsError::Network(e.to_string()))?;

        let status = json["status"].as_i64().unwrap_or(0);
        if status == 1 {
            let mid = json["data"]["messageId"]
                .as_i64()
                .map(|x| x.to_string())
                .unwrap_or_default();
            Ok(SmsReceipt { message_id: mid })
        } else {
            let message = json["message"]
                .as_str()
                .unwrap_or("خطای نامشخص SMS.ir")
                .to_string();
            Err(SmsError::Provider {
                code: status,
                message,
            })
        }
    }
    fn name(&self) -> &'static str {
        "smsir"
    }
}

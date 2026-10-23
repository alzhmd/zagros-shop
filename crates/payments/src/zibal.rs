use crate::{GwError, PayInit, PayRequest, PayResult, PaymentGateway};
use async_trait::async_trait;
use serde_json::json;

/// درگاه زیبال (IPG). مبلغ همیشه ریال. حالت lazy برای ضدتقلب.
pub struct Zibal {
    merchant: String,
    client: reqwest::Client,
}

impl Zibal {
    pub fn new(merchant: &str) -> Self {
        Self {
            merchant: merchant.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

const REQUEST_URL: &str = "https://gateway.zibal.ir/v1/request";
const VERIFY_URL: &str = "https://gateway.zibal.ir/v1/verify";
const START_URL: &str = "https://gateway.zibal.ir/start/";

#[async_trait]
impl PaymentGateway for Zibal {
    fn name(&self) -> &'static str {
        "zibal"
    }

    async fn request(&self, r: &PayRequest) -> Result<PayInit, GwError> {
        if r.amount_rial < 1000 {
            return Err(GwError::InvalidAmount);
        }
        let mut body = json!({
            "merchant": self.merchant,
            "amount": r.amount_rial,
            "callbackUrl": r.callback_url,
            "orderId": r.order_id,
            "description": r.description,
            "lazy": true,    // تأیید-یا-بازگشت-خودکار: پول فقط با تأیید سرور capture می‌شود
        });
        if let Some(m) = &r.mobile {
            body["mobile"] = json!(m);
        }

        let resp = self
            .client
            .post(REQUEST_URL)
            .json(&body)
            .send()
            .await
            .map_err(|e| GwError::Network(e.to_string()))?;
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GwError::Network(e.to_string()))?;

        let result = v["result"].as_i64().unwrap_or(0);
        if result == 100 {
            let track_id = v["trackId"]
                .as_i64()
                .map(|x| x.to_string())
                .unwrap_or_default();
            Ok(PayInit {
                redirect_url: format!("{START_URL}{track_id}"),
                gateway_token: track_id,
            })
        } else {
            Err(GwError::Gateway {
                code: result,
                message: format!("درخواست زیبال ناموفق (کد {result})"),
            })
        }
    }

    async fn verify(&self, token: &str, _amount_rial: u64) -> Result<PayResult, GwError> {
        let body =
            json!({ "merchant": self.merchant, "trackId": token.parse::<i64>().unwrap_or(0) });
        let resp = self
            .client
            .post(VERIFY_URL)
            .json(&body)
            .send()
            .await
            .map_err(|e| GwError::Network(e.to_string()))?;
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GwError::Network(e.to_string()))?;

        let result = v["result"].as_i64().unwrap_or(-1);
        let status = v["status"].as_i64().unwrap_or(-1);
        // result: 100 موفق، 201 قبلاً تأیید شده. status: 1 = پرداخت و تأیید شده.
        let already = result == 201;
        if (result == 100 || result == 201) && (status == 1 || already) {
            Ok(PayResult {
                paid: true,
                already_verified: already,
                ref_id: v["refNumber"]
                    .as_i64()
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                card_pan: v["cardNumber"].as_str().map(|s| s.to_string()),
            })
        } else {
            tracing::warn!(result, status, "تأیید زیبال ناموفق");
            Ok(PayResult {
                paid: false,
                already_verified: false,
                ref_id: String::new(),
                card_pan: None,
            })
        }
    }
}

use crate::{GwError, PayInit, PayRequest, PayResult, PaymentGateway};
use async_trait::async_trait;
use serde_json::json;

/// درگاه زرین‌پال (REST API v4). مبلغ به ریال؛ currency صریح "IRR".
pub struct Zarinpal {
    merchant_id: String,
    client: reqwest::Client,
}

impl Zarinpal {
    pub fn new(merchant_id: &str) -> Self {
        Self {
            merchant_id: merchant_id.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

const REQUEST_URL: &str = "https://payment.zarinpal.com/pg/v4/payment/request.json";
const VERIFY_URL: &str = "https://payment.zarinpal.com/pg/v4/payment/verify.json";
const STARTPAY: &str = "https://www.zarinpal.com/pg/StartPay/";

#[async_trait]
impl PaymentGateway for Zarinpal {
    fn name(&self) -> &'static str {
        "zarinpal"
    }

    async fn request(&self, r: &PayRequest) -> Result<PayInit, GwError> {
        if r.amount_rial < 1000 {
            return Err(GwError::InvalidAmount);
        }
        let mut body = json!({
            "merchant_id": self.merchant_id,
            "currency": "IRR",             // صریح: مبلغ به ریال است
            "amount": r.amount_rial,
            "callback_url": r.callback_url,
            "description": r.description,
        });
        if let Some(m) = &r.mobile {
            body["metadata"] = json!({ "mobile": m });
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

        let code = v["data"]["code"].as_i64().unwrap_or(0);
        if code == 100 {
            let authority = v["data"]["authority"].as_str().unwrap_or("").to_string();
            Ok(PayInit {
                redirect_url: format!("{STARTPAY}{authority}"),
                gateway_token: authority,
            })
        } else {
            let message = v["errors"]["message"]
                .as_str()
                .unwrap_or("خطای درخواست زرین‌پال")
                .to_string();
            Err(GwError::Gateway { code, message })
        }
    }

    async fn verify(&self, token: &str, amount_rial: u64) -> Result<PayResult, GwError> {
        let body = json!({
            "merchant_id": self.merchant_id,
            "amount": amount_rial,
            "authority": token,
        });
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

        let code = v["data"]["code"].as_i64().unwrap_or(-1);
        // 100 = موفق، 101 = قبلاً تأیید شده (idempotent)
        if code == 100 || code == 101 {
            Ok(PayResult {
                paid: true,
                already_verified: code == 101,
                ref_id: v["data"]["ref_id"]
                    .as_i64()
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                card_pan: v["data"]["card_pan"].as_str().map(|s| s.to_string()),
            })
        } else {
            let message = v["errors"]["message"]
                .as_str()
                .unwrap_or("تأیید زرین‌پال ناموفق")
                .to_string();
            // در صورت خطای قطعی، paid=false برمی‌گردانیم نه خطا، تا فلو تصمیم بگیرد.
            tracing::warn!(code, message = %message, "تأیید زرین‌پال ناموفق");
            Ok(PayResult {
                paid: false,
                already_verified: false,
                ref_id: String::new(),
                card_pan: None,
            })
        }
    }
}

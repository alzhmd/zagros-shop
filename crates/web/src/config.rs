use serde::Deserialize;

/// پیکربندی برنامه از `config/config.toml` و متغیرهای محیطی (پیشوند `ZAGROS__`).
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub sms: SmsConfig,
    #[serde(default)]
    pub payment: PaymentConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub email: EmailConfig,
    #[serde(default)]
    pub admin: AdminConfig,
}

/// بوت‌استرپ مدیر اول. موبایلِ این کاربر در هر راه‌اندازی به نقش admin ارتقا می‌یابد.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminConfig {
    /// موبایل مدیر (مثلاً 09xxxxxxxxx). خالی = بدون بوت‌استرپ.
    #[serde(default)]
    pub mobile: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from: String,
}
fn default_smtp_port() -> u16 {
    587
}
impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: String::new(), // خالی = حالت کنسول (فقط لاگ)
            smtp_port: 587,
            smtp_user: String::new(),
            smtp_pass: String::new(),
            from: "زاگرس <no-reply@zagros.local>".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_cookie_secret")]
    pub cookie_secret: String,
}

fn default_base_url() -> String {
    "http://localhost:8080".to_string()
}
fn default_cookie_secret() -> String {
    // فقط برای توسعه؛ در تولید حتماً از طریق ENV یک secret تصادفی بلند بده.
    "zagros-dev-only-cookie-secret-change-me-in-production-please-now".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmsConfig {
    pub provider: String, // none | kavenegar | smsir
    pub api_key: String,
    pub template: String,
}
impl Default for SmsConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            api_key: String::new(),
            template: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentConfig {
    pub provider: String, // zarinpal | zibal
    pub merchant: String,
    pub enabled: bool,
}
impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            provider: "zibal".into(),
            merchant: "zibal".into(), // مرچنت تستی زیبال
            enabled: false,           // تا قبل از خرید درگاه، آنلاین غیرفعال
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    pub enabled: bool,
}
impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            enabled: false, // توکن از طریق ENV/config داده می‌شود؛ پیش‌فرض خاموش
        }
    }
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config/config").required(false))
            // override محلی (gitignored) — جای امن برای رازها مثل توکن ربات.
            .add_source(config::File::with_name("config/config.local").required(false))
            .add_source(
                config::Environment::with_prefix("ZAGROS")
                    .separator("__")
                    .try_parsing(true),
            )
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.base_url", "http://localhost:8080")?
            .set_default("server.cookie_secret", default_cookie_secret())?
            .set_default(
                "database.url",
                "postgres://zagros:zagros_dev_2026@127.0.0.1:5432/zagros",
            )?
            .set_default("sms.provider", "none")?
            .set_default("sms.api_key", "")?
            .set_default("sms.template", "")?
            .set_default("payment.provider", "zibal")?
            .set_default("payment.merchant", "zibal")?
            .set_default("payment.enabled", false)?
            .set_default("telegram.token", "")?
            .set_default("telegram.enabled", false)?
            .set_default("admin.mobile", "")?
            .build()?;
        Ok(cfg.try_deserialize()?)
    }
}

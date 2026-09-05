use axum_extra::extract::cookie::Key;
use db::Db;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use uuid::Uuid;

/// حالت مشترک برنامه.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    /// تنظیمات قابل‌ویرایش از پنل ادمین (کش‌شده، زنده).
    pub settings: SettingsCache,
    pub base_url: Arc<str>,
    pub cookie_key: Key,
    pub sms: Arc<dyn sms::SmsSender>,
    pub mailer: crate::mailer::Mailer,
    pub payment: Arc<PaymentConfig>,
    pub chat: ChatHub,
    /// هندل ربات تلگرام برای ارسال اعلان؛ None اگر ربات غیرفعال باشد.
    pub bot: Option<bot::BotHandle>,
    /// نام کاربری ربات (بدون @) برای ساخت لینک عمیق t.me؛ خالی اگر نامشخص.
    pub bot_username: Arc<str>,
    /// کش درون‌حافظه‌ای صفحه‌ی اصلی (محصولات منتخب + دسته‌ها).
    pub home_cache: TtlCache<HomeData>,
}

/// داده‌های کش‌شده‌ی صفحه‌ی اصلی.
#[derive(Clone)]
pub struct HomeData {
    pub banners: Vec<db::banners::Banner>,
    pub strip: Option<db::banners::Banner>,
    pub featured: Vec<db::models::ProductCard>,
    pub deals: Vec<db::models::ProductCard>,
    pub popular: Vec<db::models::ProductCard>,
    pub newest: Vec<db::models::ProductCard>,
    pub categories: Vec<db::models::Category>,
}

/// کش تک‌مقداری با TTL — سبک، بدون وابستگی (به‌جای moka، به‌خاطر محدودیت دیسک).
/// با `RwLock` خواندن‌های هم‌زمان آزادند؛ نوشتن فقط هنگام تازه‌سازی.
pub struct TtlCache<T> {
    ttl: Duration,
    inner: Arc<RwLock<Option<(Instant, T)>>>,
}

impl<T> Clone for TtlCache<T> {
    fn clone(&self) -> Self {
        Self {
            ttl: self.ttl,
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Clone> TtlCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Arc::new(RwLock::new(None)),
        }
    }

    /// مقدار تازه را برمی‌گرداند؛ اگر منقضی/خالی بود `None`.
    pub fn get(&self) -> Option<T> {
        let guard = self.inner.read().ok()?;
        match guard.as_ref() {
            Some((at, val)) if at.elapsed() < self.ttl => Some(val.clone()),
            _ => None,
        }
    }

    /// مقدار را با مهر زمانی فعلی ذخیره می‌کند.
    pub fn set(&self, val: T) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some((Instant::now(), val));
        }
    }

    /// باطل‌کردن دستی کش — قلاب آماده برای زمانی که پنل CRUD محصول/دسته اضافه شود.
    /// (فعلاً CRUD محصول از طریق migration/seed است؛ تازه‌سازی با TTL ۶۰ ثانیه انجام می‌شود.)
    #[allow(dead_code)]
    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = None;
        }
    }
}

/// هاب چت زنده‌ی درون‌حافظه‌ای: برای هر جلسه یک کانال broadcast.
/// چون اپ تک‌پروسه است نیازی به Redis/بروکر نیست؛ پیام‌ها در دیتابیس هم ذخیره می‌شوند.
#[derive(Clone, Default)]
pub struct ChatHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}

impl ChatHub {
    /// فرستنده‌ی کانال یک جلسه را برمی‌گرداند (در صورت نبود، می‌سازد).
    pub fn channel(&self, session: Uuid) -> broadcast::Sender<String> {
        let mut map = self.channels.lock().expect("قفل هاب چت آسیب دیده");
        map.entry(session)
            .or_insert_with(|| broadcast::channel(128).0)
            .clone()
    }
}

/// پیکربندی فعال پرداخت.
pub struct PaymentConfig {
    pub provider: String,
    pub merchant: String,
    pub enabled: bool,
}

/// کش تنظیمات سایت — از جدول `settings` خوانده می‌شود و با هر ذخیره تازه می‌شود.
/// مقادیر قابل‌ویرایش زمان اجرا؛ رازها اینجا نیستند.
#[derive(Clone)]
pub struct SettingsCache {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl SettingsCache {
    /// بارگذاری اولیه از دیتابیس.
    pub async fn load(db: &Db) -> Self {
        let map = db::settings::get_all(db).await.unwrap_or_default();
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }

    /// تازه‌سازی کش از دیتابیس (پس از ذخیره‌ی ادمین).
    pub async fn reload(&self, db: &Db) {
        if let Ok(map) = db::settings::get_all(db).await {
            if let Ok(mut g) = self.inner.write() {
                *g = map;
            }
        }
    }

    fn raw(&self, key: &str) -> Option<String> {
        self.inner.read().ok().and_then(|g| g.get(key).cloned())
    }
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.raw(key)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string())
    }
    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.raw(key).and_then(|s| s.trim().parse().ok()).unwrap_or(default)
    }
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.raw(key).as_deref() {
            Some("true") | Some("1") | Some("on") => true,
            Some("false") | Some("0") => false,
            _ => default,
        }
    }

    // --- دسترسی‌های نوع‌دار ---
    pub fn site_name(&self) -> String {
        self.get_or("site.name", "زاگرس — جهان کولاک")
    }
    pub fn contact_phone(&self) -> String {
        self.get_or("site.contact_phone", "")
    }
    pub fn contact_address(&self) -> String {
        self.get_or("site.contact_address", "استان زنجان — ارسال به سراسر کشور")
    }
    pub fn shipping_toman(&self) -> i64 {
        self.get_i64("shipping.flat_toman", 0).max(0)
    }
    pub fn shipping_rial(&self) -> i64 {
        self.shipping_toman() * 10
    }
    pub fn cod_province(&self) -> String {
        self.get_or("cod.province", "زنجان")
    }
    pub fn rial_per_point(&self) -> i64 {
        let v = self.get_i64("loyalty.rial_per_point", 100_000);
        if v <= 0 {
            100_000
        } else {
            v
        }
    }
    /// ارزش ریالی هر امتیاز هنگام خرج‌کردن (تخفیف). پیش‌فرض: هر امتیاز ۱۰٬۰۰۰ ریال (۱٬۰۰۰ تومان).
    pub fn redeem_rial_per_point(&self) -> i64 {
        let v = self.get_i64("loyalty.redeem_rial_per_point", 10_000);
        if v <= 0 {
            10_000
        } else {
            v
        }
    }
    pub fn payments_enabled(&self) -> bool {
        self.get_bool("payments.enabled", false)
    }

    /// روش‌های ارسالِ فعال (با هزینه و زمان تقریبی). اگر هیچ‌کدام فعال نبود،
    /// یک روش پیش‌فرض «ارسال» با هزینه‌ی پایه‌ی قدیمی برمی‌گردد.
    pub fn shipping_methods(&self) -> Vec<ShippingMethod> {
        let keys = [
            ("normal", "ارسال عادی", 0),
            ("express", "ارسال پیشتاز", 50_000),
            ("instant", "ارسال فوری (همان روز)", 120_000),
        ];
        let mut out = Vec::new();
        for (key, def_label, def_toman) in keys {
            if self.get_bool(&format!("shipping.{key}.enabled"), key == "normal") {
                let toman = self.get_i64(&format!("shipping.{key}.toman"), def_toman).max(0);
                out.push(ShippingMethod {
                    key: key.to_string(),
                    label: self.get_or(&format!("shipping.{key}.label"), def_label),
                    rial: toman * 10,
                    eta: self.get_or(&format!("shipping.{key}.eta"), ""),
                });
            }
        }
        if out.is_empty() {
            out.push(ShippingMethod {
                key: "normal".into(),
                label: "ارسال".into(),
                rial: self.shipping_rial(),
                eta: String::new(),
            });
        }
        out
    }

    /// یافتن یک روش ارسال با کلید (برای اعتبارسنجی هنگام ثبت سفارش).
    pub fn shipping_method(&self, key: &str) -> Option<ShippingMethod> {
        self.shipping_methods().into_iter().find(|m| m.key == key)
    }
}

/// یک روش ارسالِ قابل‌انتخاب در تسویه.
#[derive(Clone, Debug)]
pub struct ShippingMethod {
    pub key: String,
    pub label: String,
    pub rial: i64,
    pub eta: String,
}

impl ShippingMethod {
    pub fn cost_fa(&self) -> String {
        if self.rial == 0 {
            "رایگان".to_string()
        } else {
            domain::Rial(self.rial).format_toman_fa()
        }
    }
}

impl AppState {
    pub fn new(
        db: Db,
        settings: SettingsCache,
        base_url: &str,
        cookie_secret: &str,
        sms: Arc<dyn sms::SmsSender>,
        mailer: crate::mailer::Mailer,
        payment: PaymentConfig,
    ) -> Self {
        // کلید کوکی از secret تنظیمات (باید حداقل ۶۴ بایت باشد).
        let key = derive_key(cookie_secret);
        Self {
            db,
            settings,
            base_url: Arc::from(base_url.trim_end_matches('/')),
            cookie_key: key,
            sms,
            mailer,
            payment: Arc::new(payment),
            chat: ChatHub::default(),
            bot: None,
            bot_username: Arc::from(""),
            // TTL کوتاه: صفحه‌ی اصلی حداکثر ۶۰ ثانیه قدیمی می‌شود.
            home_cache: TtlCache::new(Duration::from_secs(60)),
        }
    }

    /// نام سایت (از کش تنظیمات؛ زنده).
    pub fn site_name(&self) -> String {
        self.settings.site_name()
    }

    /// اتصال هندل ربات تلگرام و نام کاربری آن (builder).
    pub fn with_bot(mut self, handle: Option<bot::BotHandle>, username: &str) -> Self {
        self.bot = handle;
        self.bot_username = Arc::from(username);
        self
    }
}

/// اجازه می‌دهد `SignedCookieJar` کلید را از state بگیرد.
impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

fn derive_key(secret: &str) -> Key {
    let bytes = secret.as_bytes();
    if bytes.len() >= 64 {
        Key::from(bytes)
    } else {
        // گسترش امن secret کوتاه به ۶۴ بایت با تکرار (فقط برای توسعه؛ در تولید secret بلند بده).
        let mut buf = Vec::with_capacity(64);
        while buf.len() < 64 {
            buf.extend_from_slice(bytes);
        }
        buf.truncate(64);
        Key::from(&buf)
    }
}

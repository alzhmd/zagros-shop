mod auth;
mod config;
mod csrf;
mod error;
mod extract;
mod filters;
mod loyalty_hook;
mod mailer;
mod routes;
mod state;
mod upload;
mod web;

use anyhow::Context;
use config::Settings;
use state::{AppState, PaymentConfig, SettingsCache};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let settings = Settings::load().context("بارگذاری پیکربندی شکست خورد")?;
    tracing::info!("اتصال به دیتابیس…");
    let db = db::connect(&settings.database.url)
        .await
        .context("اتصال به Postgres شکست خورد")?;

    tracing::info!("اجرای migrationها…");
    db::migrate(&db).await.context("migration شکست خورد")?;

    // بوت‌استرپ مدیر اول: اگر موبایل مدیر در پیکربندی/ENV داده شده باشد، آن کاربر
    // در هر راه‌اندازی به نقش admin ارتقا می‌یابد (سپس با OTP وارد می‌شود).
    if let Some(mobile) = auth::normalize_mobile(&settings.admin.mobile) {
        match db::accounts::ensure_admin(&db, &mobile).await {
            Ok(()) => tracing::info!(%mobile, "کاربر مدیر آماده شد (نقش admin)"),
            Err(e) => tracing::error!(error = %e, "بوت‌استرپ مدیر شکست خورد"),
        }
    }

    // کش تنظیمات سایت (قابل‌ویرایش از پنل ادمین، زنده).
    let settings_cache = SettingsCache::load(&db).await;

    // فرستنده‌ی پیامک (در توسعه: کنسول).
    let sms = sms::build(
        &settings.sms.provider,
        &settings.sms.api_key,
        &settings.sms.template,
    );

    let payment = PaymentConfig {
        provider: settings.payment.provider.clone(),
        merchant: settings.payment.merchant.clone(),
        enabled: settings.payment.enabled,
    };

    // فرستنده‌ی ایمیل (در صورت نبود SMTP: حالت کنسول/لاگ).
    let mailer = mailer::Mailer::new(
        &settings.email.smtp_host,
        settings.email.smtp_port,
        &settings.email.smtp_user,
        &settings.email.smtp_pass,
        &settings.email.from,
    );

    // ربات تلگرام (فاز ۵): در صورت فعال بودن، هندل اعلان ساخته و حلقه‌ی polling
    // در یک تسک جدا اجرا می‌شود — یک باینری، چند نقش.
    let (bot_handle, bot_username) =
        if settings.telegram.enabled && !settings.telegram.token.is_empty() {
            let handle = bot::BotHandle::new(&settings.telegram.token);
            // نام کاربری ربات را یک‌بار در راه‌اندازی می‌گیریم (برای لینک t.me).
            let username = handle.username().await;
            let token = settings.telegram.token.clone();
            let bot_db = db.clone();
            tokio::spawn(async move {
                bot::run(token, bot_db).await;
            });
            (Some(handle), username)
        } else {
            tracing::info!("ربات تلگرام غیرفعال است (telegram.enabled=false یا توکن خالی)");
            (None, String::new())
        };

    let state = AppState::new(
        db,
        settings_cache,
        &settings.server.base_url,
        &settings.server.cookie_secret,
        Arc::from(sms),
        mailer,
        payment,
    )
    .with_bot(bot_handle, &bot_username);
    let app = routes::router(state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("bind روی {addr} شکست خورد"))?;
    tracing::info!("سرور زاگرس روی http://{addr} بالا آمد");

    axum::serve(listener, app).await.context("سرور متوقف شد")?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

use crate::auth::{self, SESSION_COOKIE};
use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar};
use chrono::{Duration, Utc};
use serde::Deserialize;

// ---------- صفحه‌ی ورود (مرحله ۱: گرفتن موبایل) ----------

#[derive(Template)]
#[template(path = "account/login.html")]
struct LoginTemplate {
    site_name: String,
    error: Option<String>,
}

pub async fn login_page(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(LoginTemplate {
        site_name: s.site_name(),
        error: None,
    })
}

#[derive(Deserialize)]
pub struct SendOtpForm {
    pub mobile: String,
}

/// مرحله ۱: ارسال کد OTP.
pub async fn send_otp(
    State(s): State<AppState>,
    Form(form): Form<SendOtpForm>,
) -> AppResult<axum::response::Response> {
    let Some(mobile) = auth::normalize_mobile(&form.mobile) else {
        return Ok(render::<LoginTemplate>(LoginTemplate {
            site_name: s.site_name(),
            error: Some("شماره موبایل نامعتبر است.".into()),
        })?
        .into_response());
    };

    // محدودیت نرخ: حداکثر ۳ کد در ۵ دقیقه.
    let recent = db::accounts::recent_otp_count(&s.db, &mobile, 5).await?;
    if recent >= 3 {
        return Ok(render::<LoginTemplate>(LoginTemplate {
            site_name: s.site_name(),
            error: Some("تعداد درخواست‌ها زیاد است. چند دقیقه بعد دوباره تلاش کنید.".into()),
        })?
        .into_response());
    }

    let code = auth::generate_otp();
    let code_hash = auth::hash_secret(&code).map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    let expires = Utc::now() + Duration::minutes(2);
    db::accounts::store_otp(&s.db, &mobile, &code_hash, expires).await?;

    // ارسال از طریق آداپتور پیامک فعال (در توسعه: ConsoleSms کد را لاگ می‌کند).
    if let Err(e) = s.sms.send_otp(&mobile, &code).await {
        tracing::error!(error = %e, "ارسال پیامک ناموفق");
    }

    // نمایش صفحه‌ی تأیید کد.
    render(VerifyTemplate {
        site_name: s.site_name(),
        mobile,
        error: None,
    })
    .map(|h| h.into_response())
}

// ---------- مرحله ۲: تأیید کد ----------

#[derive(Template)]
#[template(path = "account/verify.html")]
struct VerifyTemplate {
    site_name: String,
    mobile: String,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifyForm {
    pub mobile: String,
    pub code: String,
}

pub async fn verify_otp(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<VerifyForm>,
) -> AppResult<axum::response::Response> {
    let mobile = auth::normalize_mobile(&form.mobile).unwrap_or(form.mobile.clone());
    let code: String = form.code.chars().filter(|c| c.is_ascii_digit()).collect();

    let otp = db::accounts::latest_otp(&s.db, &mobile).await?;
    let err = |msg: &str| -> AppResult<axum::response::Response> {
        render::<VerifyTemplate>(VerifyTemplate {
            site_name: s.site_name(),
            mobile: mobile.clone(),
            error: Some(msg.to_string()),
        })
        .map(|h| h.into_response())
    };

    let Some(otp) = otp else {
        return err("کدی یافت نشد. دوباره درخواست دهید.");
    };
    if otp.consumed {
        return err("این کد قبلاً استفاده شده است.");
    }
    if otp.expires_at < Utc::now() {
        return err("کد منقضی شده است. دوباره درخواست دهید.");
    }
    if otp.attempts >= 5 {
        return err("تعداد تلاش‌ها زیاد است. کد جدید بگیرید.");
    }
    if !auth::verify_secret(&code, &otp.code_hash) {
        db::accounts::increment_otp_attempts(&s.db, otp.id).await?;
        return err("کد نادرست است.");
    }

    // موفق: مصرف کد و ساخت/یافتن کاربر.
    db::accounts::mark_otp_consumed(&s.db, otp.id).await?;
    let is_new = db::accounts::find_by_mobile(&s.db, &mobile)
        .await?
        .is_none();
    let user = db::accounts::upsert_by_mobile(&s.db, &mobile).await?;

    // کارت هدیه‌ی خوش‌آمدگویی برای کاربر جدید (در صورت فعال‌بودن در تنظیمات).
    if is_new && s.settings.get_bool("giftcard.signup_enabled", false) {
        let toman = s.settings.get_i64("giftcard.signup_toman", 50_000).max(0);
        if toman > 0 {
            let code = auth::generate_link_code();
            let days = s.settings.get_i64("giftcard.signup_days", 90).max(1);
            let expires = Utc::now() + Duration::days(days);
            if db::gift_cards::issue(&s.db, &code, toman * 10, Some(user.id), Some(expires))
                .await
                .is_ok()
            {
                let title = "کارت هدیه‌ی خوش‌آمدگویی 🎁".to_string();
                let body =
                    format!("کد {code} را در کیف پول وارد کنید و موجودی هدیه را دریافت کنید.");
                let _ = db::notifications::create(
                    &s.db,
                    user.id,
                    "coupon",
                    &title,
                    Some(&body),
                    Some("/account/wallet"),
                )
                .await;
            }
        }
    }

    // ساخت نشست سمت‌سرور + کوکیِ امضاشده با توکن نشست.
    let token = auth::generate_session_token();
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    let ip = client_ip(&headers);
    // در صورت خطای ساخت نشست، ورود را مسدود نمی‌کنیم (کوکی به‌هرحال معتبر است).
    let _ = db::sessions::create(&s.db, &token, user.id, ua, ip.as_deref()).await;

    let mut cookie = Cookie::new(SESSION_COOKIE, token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Lax);
    cookie.set_max_age(time::Duration::days(30));
    let jar = jar.add(cookie);

    Ok((jar, Redirect::to("/account")).into_response())
}

/// استخراج آی‌پیِ واقعیِ کلاینت از هدرهای پراکسی (nginx: X-Forwarded-For / X-Real-IP).
fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

// ---------- داشبورد حساب ----------

#[derive(Template)]
#[template(path = "account/dashboard.html")]
struct DashboardTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    recent_orders: Vec<db::orders::Order>,
    order_count_fa: String,
    return_count_fa: String,
    repair_count_fa: String,
    wishlist_count_fa: String,
    address_count_fa: String,
    wallet_fa: String,
    // ---- نمای کلیِ هوشمند ----
    active_order: Option<db::orders::Order>,
    profile_pct: i64,
    profile_pct_fa: String,
    tier: String,
    tier_key: &'static str,
    tier_next: Option<String>,
    to_next_fa: String,
    progress_pct: i64,
}

/// داشبورد/نمای کلی حساب (مثل دیجی‌کالا): کارت‌های خلاصه + سفارش‌های اخیر.
pub async fn dashboard(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let orders = db::orders::list_for_user(&s.db, user.id).await?;
    let order_count = orders.len() as i64;
    // جدیدترین سفارشِ «در جریان» برای بنر نیازمند توجه (لیست بر اساس created_at نزولی است).
    let active_order = orders.iter().find(|o| o.is_active()).cloned();
    let recent_orders: Vec<db::orders::Order> = orders.into_iter().take(4).collect();
    let return_count = db::returns::list_for_user(&s.db, user.id)
        .await
        .map(|v| v.len() as i64)
        .unwrap_or(0);
    let repair_count = db::repairs::list_for_user(&s.db, user.id)
        .await
        .map(|v| v.len() as i64)
        .unwrap_or(0);
    let wishlist_count = db::wishlist::count(&s.db, user.id).await.unwrap_or(0);
    let address_count = db::accounts::list_addresses(&s.db, user.id)
        .await
        .map(|v| v.len() as i64)
        .unwrap_or(0);
    let fa = |n: i64| crate::filters::to_fa_digits(&n.to_string());

    // سطح باشگاه — همان منطقِ صفحه‌ی باشگاه، تا داشبورد و باشگاه هم‌خوان بمانند.
    let silver = s.settings.get_i64("club.silver_threshold", 200);
    let gold = s.settings.get_i64("club.gold_threshold", 1000);
    let (tier, tier_next, to_next, progress_pct) = club_tier(user.points, silver, gold);
    let tier_key = if user.points >= gold {
        "gold"
    } else if user.points >= silver {
        "silver"
    } else {
        "bronze"
    };

    // درصد تکمیل پروفایل (۵ سیگنال): نام، ایمیل، تاریخ تولد، جنسیت، حداقل یک آدرس.
    let mut filled = 0i64;
    if user
        .full_name
        .as_deref()
        .map_or(false, |v| !v.trim().is_empty())
    {
        filled += 1;
    }
    if user
        .email
        .as_deref()
        .map_or(false, |v| !v.trim().is_empty())
    {
        filled += 1;
    }
    if user.birthdate.is_some() {
        filled += 1;
    }
    if user.gender.is_some() {
        filled += 1;
    }
    if address_count > 0 {
        filled += 1;
    }
    let profile_pct = filled * 100 / 5;

    render(DashboardTemplate {
        site_name: s.site_name(),
        points_fa: fa(user.points),
        recent_orders,
        order_count_fa: fa(order_count),
        return_count_fa: fa(return_count),
        repair_count_fa: fa(repair_count),
        wishlist_count_fa: fa(wishlist_count),
        address_count_fa: fa(address_count),
        wallet_fa: domain::Rial(user.wallet_balance).format_toman_fa(),
        active_order,
        profile_pct,
        profile_pct_fa: fa(profile_pct),
        tier,
        tier_key,
        tier_next,
        to_next_fa: fa(to_next),
        progress_pct,
        user,
    })
    .map(|h| h.into_response())
}

// ---------- سوابق خرید (همه‌ی سفارش‌ها) ----------

#[derive(Template)]
#[template(path = "account/orders.html")]
struct OrdersTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    orders: Vec<db::orders::Order>,
}

pub async fn orders(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let orders = db::orders::list_for_user(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(OrdersTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        orders,
    })
    .map(|h| h.into_response())
}

// ---------- تعمیرات من ----------

#[derive(Template)]
#[template(path = "account/repairs.html")]
struct RepairsTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    repairs: Vec<db::repairs::RepairRequest>,
}

pub async fn repairs(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let repairs = db::repairs::list_for_user(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(RepairsTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        repairs,
    })
    .map(|h| h.into_response())
}

// ---------- باشگاه مشتریان (امتیاز + اتصال تلگرام) ----------

#[derive(Template)]
#[template(path = "account/club.html")]
struct ClubTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    history: Vec<db::loyalty::LoyaltyTx>,
    telegram_linked: bool,
    link_code: Option<String>,
    bot_username: String,
    tier: String,
    tier_key: &'static str,
    tier_next: Option<String>,
    to_next_fa: String,
    progress_pct: i64,
    silver_fa: String,
    gold_fa: String,
}

/// سطح باشگاه بر پایه‌ی امتیاز و آستانه‌های settings.
/// خروجی: (نام سطح فعلی، نام سطح بعدی، امتیاز تا سطح بعد، درصد پیشرفت).
fn club_tier(points: i64, silver: i64, gold: i64) -> (String, Option<String>, i64, i64) {
    if points >= gold {
        ("طلایی".into(), None, 0, 100)
    } else if points >= silver {
        let span = (gold - silver).max(1);
        let pct = ((points - silver) * 100 / span).clamp(0, 100);
        ("نقره‌ای".into(), Some("طلایی".into()), gold - points, pct)
    } else {
        let span = silver.max(1);
        let pct = (points * 100 / span).clamp(0, 100);
        ("برنزی".into(), Some("نقره‌ای".into()), silver - points, pct)
    }
}

pub async fn club(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let history = db::loyalty::history(&s.db, user.id, 20).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    let telegram_linked = user.telegram_id.is_some();
    let silver = s.settings.get_i64("club.silver_threshold", 200);
    let gold = s.settings.get_i64("club.gold_threshold", 1000);
    let (tier, tier_next, to_next, progress_pct) = club_tier(user.points, silver, gold);
    let tier_key = if user.points >= gold {
        "gold"
    } else if user.points >= silver {
        "silver"
    } else {
        "bronze"
    };
    let fa = |n: i64| crate::filters::to_fa_digits(&n.to_string());
    render(ClubTemplate {
        site_name: s.site_name(),
        points_fa,
        telegram_linked,
        link_code: None,
        bot_username: s.bot_username.to_string(),
        history,
        tier,
        tier_key,
        tier_next,
        to_next_fa: fa(to_next),
        progress_pct,
        silver_fa: fa(silver),
        gold_fa: fa(gold),
        user,
    })
    .map(|h| h.into_response())
}

/// تولید کد اتصال تلگرام (POST). کد کوتاه با انقضای ۱۵ دقیقه.
pub async fn generate_link_code(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let code = auth::generate_link_code();
    let expires = Utc::now() + Duration::minutes(15);
    db::accounts::create_link_code(&s.db, user.id, &code, expires).await?;

    let history = db::loyalty::history(&s.db, user.id, 20).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    let telegram_linked = user.telegram_id.is_some();
    let silver = s.settings.get_i64("club.silver_threshold", 200);
    let gold = s.settings.get_i64("club.gold_threshold", 1000);
    let (tier, tier_next, to_next, progress_pct) = club_tier(user.points, silver, gold);
    let tier_key = if user.points >= gold {
        "gold"
    } else if user.points >= silver {
        "silver"
    } else {
        "bronze"
    };
    let fa = |n: i64| crate::filters::to_fa_digits(&n.to_string());
    render(ClubTemplate {
        site_name: s.site_name(),
        points_fa,
        telegram_linked,
        link_code: Some(code),
        bot_username: s.bot_username.to_string(),
        history,
        tier,
        tier_key,
        tier_next,
        to_next_fa: fa(to_next),
        progress_pct,
        silver_fa: fa(silver),
        gold_fa: fa(gold),
        user,
    })
    .map(|h| h.into_response())
}

// ---------- کیف پول ----------

#[derive(Template)]
#[template(path = "account/wallet.html")]
struct WalletTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    balance_fa: String,
    history: Vec<db::wallet::WalletTx>,
    gift_msg: Option<String>,
    gift_ok: bool,
}

#[derive(Deserialize)]
pub struct WalletQuery {
    pub gift: Option<String>,
}

pub async fn wallet(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Query(q): axum::extract::Query<WalletQuery>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let history = db::wallet::history(&s.db, user.id, 50)
        .await
        .unwrap_or_default();
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    let balance_fa = domain::Rial(user.wallet_balance).format_toman_fa();
    let (gift_msg, gift_ok) = match q.gift.as_deref() {
        Some("ok") => (
            Some("کارت هدیه با موفقیت به کیف پول شما اضافه شد.".to_string()),
            true,
        ),
        Some("bad") => (
            Some("کد کارت هدیه نامعتبر یا قبلاً استفاده شده است.".to_string()),
            false,
        ),
        Some("empty") => (Some("کد کارت هدیه را وارد کنید.".to_string()), false),
        _ => (None, false),
    };
    render(WalletTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        balance_fa,
        history,
        gift_msg,
        gift_ok,
    })
    .map(|h| h.into_response())
}

// ---------- اعلان‌ها ----------

#[derive(Template)]
#[template(path = "account/notifications.html")]
struct NotificationsTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    items: Vec<db::notifications::Notification>,
}

pub async fn notifications(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let items = db::notifications::list_for_user(&s.db, user.id, 100)
        .await
        .unwrap_or_default();
    // باز کردن صفحه = خواندن همه.
    let _ = db::notifications::mark_all_read(&s.db, user.id).await;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(NotificationsTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        items,
    })
    .map(|h| h.into_response())
}

#[derive(Deserialize)]
pub struct StockAlertForm {
    pub variant_id: uuid::Uuid,
    pub slug: Option<String>,
}

/// ثبت هشدار موجودشدن یک تنوع (نیازمند ورود).
pub async fn subscribe_stock(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<StockAlertForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    db::notifications::subscribe_stock(&s.db, user.id, f.variant_id).await?;
    let back = f
        .slug
        .map(|sl| format!("/product/{sl}"))
        .unwrap_or_else(|| "/".into());
    Ok(Redirect::to(&back).into_response())
}

// ---------- ویرایش پروفایل ----------

#[derive(Template)]
#[template(path = "account/profile.html")]
struct ProfileTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    saved: bool,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct SavedQuery {
    pub saved: Option<String>,
}

pub async fn profile_page(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Query(q): axum::extract::Query<SavedQuery>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(ProfileTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        saved: q.saved.as_deref() == Some("1"),
        error: None,
    })
    .map(|h| h.into_response())
}

#[derive(Deserialize)]
pub struct ProfileForm {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
}

pub async fn update_profile(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<ProfileForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let full_name = f
        .full_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let email = f.email.as_deref().map(str::trim).filter(|s| !s.is_empty());
    // اعتبارسنجی سبک ایمیل.
    if let Some(e) = email {
        if !e.contains('@') || !e.contains('.') {
            let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
            return render(ProfileTemplate {
                site_name: s.site_name(),
                user,
                points_fa,
                saved: false,
                error: Some("ایمیل نامعتبر است.".into()),
            })
            .map(|h| h.into_response());
        }
    }
    let birthdate = f
        .birthdate
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let gender = f
        .gender
        .as_deref()
        .map(str::trim)
        .filter(|g| *g == "male" || *g == "female");
    db::accounts::update_profile(&s.db, user.id, full_name, email, birthdate, gender).await?;
    Ok(Redirect::to("/account/profile?saved=1").into_response())
}

/// حذف حساب (نرم) — توسط خود کاربر.
pub async fn delete_account(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    db::accounts::soft_delete(&s.db, user.id).await?;
    // حذف نشست فعلی (در صورت توکن‌بودن کوکی).
    if let Some(c) = jar.get(SESSION_COOKIE) {
        let _ = db::sessions::delete(&s.db, c.value()).await;
    }
    let mut cookie = Cookie::from(SESSION_COOKIE);
    cookie.set_path("/");
    let jar = jar.remove(cookie);
    Ok((jar, Redirect::to("/")).into_response())
}

// ---------- کارت هدیه (واریز به کیف پول) ----------

#[derive(Deserialize)]
pub struct GiftForm {
    pub code: String,
}

pub async fn redeem_gift(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<GiftForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let code = f.code.trim().to_uppercase();
    if code.is_empty() {
        return Ok(Redirect::to("/account/wallet?gift=empty").into_response());
    }
    match db::gift_cards::redeem(&s.db, &code, user.id).await? {
        Ok(amount) => {
            db::wallet::add_transaction(
                &s.db,
                user.id,
                amount,
                "giftcard",
                None,
                Some("واریز کارت هدیه"),
            )
            .await?;
            Ok(Redirect::to("/account/wallet?gift=ok").into_response())
        }
        Err(_) => Ok(Redirect::to("/account/wallet?gift=bad").into_response()),
    }
}

// ---------- دفترچه‌ی آدرس ----------

#[derive(Template)]
#[template(path = "account/addresses.html")]
struct AddressesTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    addresses: Vec<db::accounts::Address>,
}

pub async fn addresses_page(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let addresses = db::accounts::list_addresses(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(AddressesTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        addresses,
    })
    .map(|h| h.into_response())
}

#[derive(Deserialize)]
pub struct AddressForm {
    pub receiver: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub postal_code: Option<String>,
    pub line: String,
    pub make_default: Option<String>,
}

pub async fn add_address(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<AddressForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    if f.receiver.trim().is_empty() || f.phone.trim().is_empty() || f.line.trim().is_empty() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "نام، تلفن و نشانی الزامی است.".into(),
        )));
    }
    let new = db::accounts::NewAddress {
        receiver: f.receiver.trim().to_string(),
        phone: f.phone.trim().to_string(),
        province: f.province.trim().to_string(),
        city: f.city.trim().to_string(),
        postal_code: f
            .postal_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from),
        line: f.line.trim().to_string(),
        make_default: matches!(f.make_default.as_deref(), Some("on")),
    };
    db::accounts::add_address(&s.db, user.id, &new).await?;
    Ok(Redirect::to("/account/addresses").into_response())
}

pub async fn set_default_address(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    db::accounts::set_default_address(&s.db, user.id, id).await?;
    Ok(Redirect::to("/account/addresses").into_response())
}

pub async fn delete_address(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    db::accounts::delete_address(&s.db, user.id, id).await?;
    Ok(Redirect::to("/account/addresses").into_response())
}

// ---------- مرکز دیدگاه‌ها و پرسش‌ها ----------

#[derive(Template)]
#[template(path = "account/reviews.html")]
struct ReviewsTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    reviews: Vec<db::reviews::MyReviewRow>,
    questions: Vec<db::qa::MyQuestionRow>,
    review_count_fa: String,
    question_count_fa: String,
    approved_count_fa: String,
    pending_count_fa: String,
}

/// صفحه‌ی «دیدگاه‌ها و پرسش‌های من».
pub async fn reviews(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let reviews = db::reviews::list_for_user(&s.db, user.id).await?;
    let questions = db::qa::list_for_user(&s.db, user.id).await?;
    let approved = reviews.iter().filter(|r| r.status == "approved").count() as i64;
    let pending = reviews.iter().filter(|r| r.status == "pending").count() as i64;
    let fa = |n: i64| crate::filters::to_fa_digits(&n.to_string());
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(ReviewsTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        review_count_fa: fa(reviews.len() as i64),
        question_count_fa: fa(questions.len() as i64),
        approved_count_fa: fa(approved),
        pending_count_fa: fa(pending),
        reviews,
        questions,
    })
    .map(|h| h.into_response())
}

// ---------- امنیت و نشست‌های فعال ----------

/// نمای یک نشست برای قالب — بدون افشای توکن (اعتبارنامه) در HTML.
struct SessionView {
    id: String,
    icon: &'static str,
    label: String,
    ip_fa: String,
    created_fa: String,
    last_seen_fa: String,
    is_current: bool,
}

#[derive(Template)]
#[template(path = "account/sessions.html")]
struct SessionsTemplate {
    site_name: String,
    user: db::accounts::User,
    points_fa: String,
    sessions: Vec<SessionView>,
    session_count_fa: String,
    has_others: bool,
}

/// صفحه‌ی «امنیت و دستگاه‌ها»: فهرست نشست‌های فعال.
pub async fn sessions(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let current = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();
    let rows = db::sessions::list_for_user(&s.db, user.id).await?;
    let has_others = rows.iter().filter(|r| r.token != current).count() > 0;
    let sessions: Vec<SessionView> = rows
        .iter()
        .map(|r| SessionView {
            id: r.id.to_string(),
            icon: r.device_icon(),
            label: r.device_label(),
            ip_fa: r.ip_fa(),
            created_fa: r.created_relative_fa(),
            last_seen_fa: r.last_seen_relative_fa(),
            is_current: r.token == current,
        })
        .collect();
    let session_count_fa = crate::filters::to_fa_digits(&sessions.len().to_string());
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    render(SessionsTemplate {
        site_name: s.site_name(),
        user,
        points_fa,
        sessions,
        session_count_fa,
        has_others,
    })
    .map(|h| h.into_response())
}

#[derive(Deserialize)]
pub struct RevokeForm {
    pub id: uuid::Uuid,
}

/// خروج از یک دستگاه مشخص (با شناسه‌ی عمومی).
pub async fn revoke_session(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
    Form(f): Form<RevokeForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let current = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();
    // اگر همین دستگاهِ فعلی لغو شود، خروج کامل + پاک‌کردن کوکی.
    if let Some(tok) = db::sessions::token_of(&s.db, f.id, user.id).await? {
        if tok == current {
            let _ = db::sessions::delete(&s.db, &current).await;
            let mut cookie = Cookie::from(SESSION_COOKIE);
            cookie.set_path("/");
            let jar = jar.remove(cookie);
            return Ok((jar, Redirect::to("/")).into_response());
        }
    }
    db::sessions::revoke(&s.db, f.id, user.id).await?;
    Ok(Redirect::to("/account/sessions").into_response())
}

/// خروج از همه‌ی دستگاه‌ها به‌جز دستگاه فعلی.
pub async fn revoke_other_sessions(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let current = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();
    db::sessions::revoke_others(&s.db, user.id, &current).await?;
    Ok(Redirect::to("/account/sessions?revoked=1").into_response())
}

pub async fn logout(State(s): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    // حذف نشست سمت‌سرور (در صورت توکن‌بودن کوکی) تا از لیست دستگاه‌ها هم برود.
    if let Some(c) = jar.get(SESSION_COOKIE) {
        let _ = db::sessions::delete(&s.db, c.value()).await;
    }
    // کوکیِ حذف باید همان مسیر «/» زمانِ ساخت را داشته باشد؛ وگرنه مرورگر
    // (به‌خاطر مسیرِ POST یعنی /account) کوکیِ اصلی را پاک نمی‌کند و کاربر همچنان وارد می‌ماند.
    let mut cookie = Cookie::from(SESSION_COOKIE);
    cookie.set_path("/");
    let jar = jar.remove(cookie);
    (jar, Redirect::to("/"))
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

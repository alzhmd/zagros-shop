use crate::auth::CART_COOKIE;
use crate::error::{AppError, AppResult};
use crate::extract::CurrentUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::cookie::SignedCookieJar;
use db::cart::CartItemView;
use db::orders::ShippingInfo;
use serde::Deserialize;

// استان مجاز پرداخت در محل و هزینه‌ی ارسال از تنظیمات (settings) خوانده می‌شوند — قابل‌ویرایش زنده.

#[derive(Template)]
#[template(path = "checkout.html")]
struct CheckoutTemplate {
    site_name: String,
    items: Vec<CartItemView>,
    subtotal_fa: String,
    shipping_fa: String,
    total_fa: String,
    free_shipping: bool,
    shipping_methods: Vec<crate::state::ShippingMethod>,
    user_mobile: String,
    user_name: String,
    cod_province: String,
    addresses: Vec<db::accounts::Address>,
    default_address_id: String,
    // فیلدهای مشترک با partials/checkout_summary.html (حالت اولیه: بدون کوپن).
    discount_fa: Option<String>,
    coupon_msg: Option<String>,
    coupon_ok: bool,
    // باشگاه: امتیاز و ارزش قابل‌استفاده.
    points_fa: String,
    points_value_fa: String,
    has_points: bool,
    // کیف پول.
    wallet_fa: String,
    has_wallet: bool,
}

pub async fn page(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    // تسویه نیاز به ورود دارد.
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(key) = jar.get(CART_COOKIE).map(|c| c.value().to_string()) else {
        return Ok(Redirect::to("/cart").into_response());
    };
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    let items = db::cart::items(&s.db, cart_id).await?;
    if items.is_empty() {
        return Ok(Redirect::to("/cart").into_response());
    }
    let subtotal: i64 = items.iter().map(|i| i.line_total()).sum();
    let shipping_methods = s.settings.shipping_methods();
    // هزینه‌ی اولیه = روش پیش‌فرض (اولین فعال).
    let shipping = shipping_methods.first().map(|m| m.rial).unwrap_or(0);
    let total = subtotal + shipping;

    let addresses = db::accounts::list_addresses(&s.db, user.id).await.unwrap_or_default();
    let default_address_id = addresses
        .iter()
        .find(|a| a.is_default)
        .map(|a| a.id.to_string())
        .unwrap_or_default();

    render(CheckoutTemplate {
        site_name: s.site_name(),
        items,
        subtotal_fa: domain::Rial(subtotal).format_toman_fa(),
        shipping_fa: domain::Rial(shipping).format_toman_fa(),
        total_fa: domain::Rial(total).format_toman_fa(),
        free_shipping: shipping == 0,
        shipping_methods,
        user_mobile: user.mobile.clone(),
        user_name: user.full_name.clone().unwrap_or_default(),
        cod_province: s.settings.cod_province(),
        addresses,
        default_address_id,
        discount_fa: None,
        coupon_msg: None,
        coupon_ok: false,
        points_fa: crate::filters::to_fa_digits(&user.points.to_string()),
        points_value_fa: domain::Rial(user.points * s.settings.redeem_rial_per_point()).format_toman_fa(),
        has_points: user.points > 0,
        wallet_fa: domain::Rial(user.wallet_balance).format_toman_fa(),
        has_wallet: user.wallet_balance > 0,
    })
    .map(|h| h.into_response())
}

// ---------- اعمال زنده‌ی کوپن (HTMX) ----------

#[derive(Template)]
#[template(path = "partials/checkout_summary.html")]
struct SummaryPartial {
    subtotal_fa: String,
    discount_fa: Option<String>,
    shipping_fa: String,
    total_fa: String,
    free_shipping: bool,
    coupon_msg: Option<String>,
    coupon_ok: bool,
}

#[derive(Deserialize)]
pub struct CouponPreviewForm {
    pub coupon_code: Option<String>,
    #[serde(default)]
    pub shipping_method: Option<String>,
}

/// پیش‌نمایش زنده‌ی تخفیف هنگام واردکردن کد (بدون ثبت سفارش).
pub async fn apply_coupon(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
    Form(f): Form<CouponPreviewForm>,
) -> AppResult<axum::response::Response> {
    let Some(key) = jar.get(CART_COOKIE).map(|c| c.value().to_string()) else {
        return Ok(Redirect::to("/cart").into_response());
    };
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    let items = db::cart::items(&s.db, cart_id).await?;
    let subtotal: i64 = items.iter().map(|i| i.line_total()).sum();
    // هزینه‌ی ارسال از روش انتخاب‌شده (برای پیش‌نمایش زنده‌ی جمع).
    let shipping = f
        .shipping_method
        .as_deref()
        .and_then(|k| s.settings.shipping_method(k))
        .map(|m| m.rial)
        .unwrap_or_else(|| s.settings.shipping_methods().first().map(|m| m.rial).unwrap_or(0));

    let mut discount: i64 = 0;
    let mut coupon_msg = None;
    let mut coupon_ok = false;
    if let Some(code) = f.coupon_code.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        match db::coupons::validate(&s.db, code, user.as_ref().map(|u| u.id), subtotal).await? {
            Ok((coupon, amount)) => {
                discount = amount;
                coupon_ok = true;
                coupon_msg = Some(format!(
                    "کد «{}» اعمال شد: {} تخفیف.",
                    coupon.code,
                    domain::Rial(amount).to_toman()
                ));
            }
            Err(reject) => coupon_msg = Some(reject.message_fa()),
        }
    }
    let total = subtotal - discount + shipping;

    render(SummaryPartial {
        subtotal_fa: domain::Rial(subtotal).format_toman_fa(),
        discount_fa: if discount > 0 {
            Some(domain::Rial(discount).format_toman_fa())
        } else {
            None
        },
        shipping_fa: domain::Rial(shipping).format_toman_fa(),
        total_fa: domain::Rial(total).format_toman_fa(),
        free_shipping: shipping == 0,
        coupon_msg,
        coupon_ok,
    })
    .map(|h| h.into_response())
}

#[derive(Deserialize)]
pub struct CheckoutForm {
    #[serde(default)]
    pub address_id: Option<String>,
    #[serde(default)]
    pub receiver: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub province: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub save_address: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub payment_method: String, // online | cod
    #[serde(default)]
    pub shipping_method: Option<String>, // کلید روش ارسال (normal/express/instant)
    #[serde(default)]
    pub coupon_code: Option<String>,
    #[serde(default)]
    pub use_points: Option<String>,
    #[serde(default)]
    pub use_wallet: Option<String>,
}

pub async fn submit(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
    Form(form): Form<CheckoutForm>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(key) = jar.get(CART_COOKIE).map(|c| c.value().to_string()) else {
        return Ok(Redirect::to("/cart").into_response());
    };
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    let items = db::cart::items(&s.db, cart_id).await?;
    if items.is_empty() {
        return Ok(Redirect::to("/cart").into_response());
    }

    // تعیین آدرس ارسال: از آدرس ذخیره‌شده یا فیلدهای واردشده.
    let ship = match form.address_id.as_deref() {
        Some(id) if id != "new" && !id.is_empty() => {
            let aid = uuid::Uuid::parse_str(id)
                .map_err(|_| domain::DomainError::Invalid("آدرس نامعتبر است.".into()))?;
            let addr = db::accounts::find_address(&s.db, user.id, aid)
                .await?
                .ok_or_else(|| domain::DomainError::Invalid("آدرس یافت نشد.".into()))?;
            ShippingInfo {
                receiver: addr.receiver,
                phone: addr.phone,
                province: addr.province,
                city: addr.city,
                address: addr.line,
            }
        }
        _ => {
            let receiver = form.receiver.clone().unwrap_or_default().trim().to_string();
            let phone = form.phone.clone().unwrap_or_default().trim().to_string();
            let province = form.province.clone().unwrap_or_default().trim().to_string();
            let city = form.city.clone().unwrap_or_default().trim().to_string();
            let address = form.address.clone().unwrap_or_default().trim().to_string();
            if receiver.is_empty() || phone.is_empty() || province.is_empty() || city.is_empty() || address.is_empty() {
                return Err(AppError::Domain(domain::DomainError::Invalid(
                    "اطلاعات آدرس ارسال کامل نیست.".into(),
                )));
            }
            // ذخیره‌ی آدرس برای خریدهای بعدی در صورت درخواست.
            if crate::routes::admin_util::checkbox(&form.save_address) {
                let _ = db::accounts::add_address(
                    &s.db,
                    user.id,
                    &db::accounts::NewAddress {
                        receiver: receiver.clone(),
                        phone: phone.clone(),
                        province: province.clone(),
                        city: city.clone(),
                        postal_code: None,
                        line: address.clone(),
                        make_default: false,
                    },
                )
                .await;
            }
            ShippingInfo {
                receiver,
                phone,
                province,
                city,
                address,
            }
        }
    };

    // روش ارسال انتخاب‌شده (با اعتبارسنجی در برابر تنظیمات؛ پیش‌فرض = اولین فعال).
    let ship_method = form
        .shipping_method
        .as_deref()
        .and_then(|k| s.settings.shipping_method(k))
        .or_else(|| s.settings.shipping_methods().into_iter().next())
        .unwrap_or_else(|| crate::state::ShippingMethod {
            key: "normal".into(),
            label: "ارسال".into(),
            rial: s.settings.shipping_rial(),
            eta: String::new(),
        });

    // پرداخت در محل فقط برای استان مجاز (از تنظیمات).
    let cod_province = s.settings.cod_province();
    let method = if form.payment_method == "cod" {
        if ship.province.trim() != cod_province {
            return Err(AppError::Domain(domain::DomainError::Invalid(format!(
                "پرداخت در محل فقط برای استان {cod_province} امکان‌پذیر است."
            ))));
        }
        "cod"
    } else {
        "online"
    };

    // کوپن تخفیف (اختیاری): اعتبارسنجی پیش از ساخت سفارش.
    let subtotal: i64 = items.iter().map(|i| i.line_total()).sum();
    let mut discount = db::orders::OrderDiscount::default();
    let mut applied_coupon: Option<db::coupons::Coupon> = None;
    if let Some(code) = form.coupon_code.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        match db::coupons::validate(&s.db, code, Some(user.id), subtotal).await? {
            Ok((coupon, amount)) => {
                discount = db::orders::OrderDiscount {
                    amount,
                    coupon_code: Some(coupon.code.clone()),
                };
                applied_coupon = Some(coupon);
            }
            Err(reject) => {
                return Err(AppError::Domain(domain::DomainError::Invalid(
                    reject.message_fa(),
                )));
            }
        }
    }

    // خرج‌کردن امتیاز باشگاه (اختیاری): تا سقف باقی‌مانده‌ی مبلغ پس از کوپن.
    let mut points_spent: i64 = 0;
    if crate::routes::admin_util::checkbox(&form.use_points) && user.points > 0 {
        let rate = s.settings.redeem_rial_per_point();
        let remaining = (subtotal - discount.amount).max(0);
        // بیشترین امتیازی که هم در توان کاربر است و هم از مبلغ باقی‌مانده بیشتر نمی‌شود.
        let max_by_amount = remaining / rate; // تعداد امتیاز کاملی که جا می‌شود
        points_spent = user.points.min(max_by_amount);
        if points_spent > 0 {
            discount.amount += points_spent * rate;
        }
    }

    // پرداخت با کیف پول (اختیاری): تا سقف باقی‌مانده‌ی مبلغ پس از کوپن و امتیاز.
    let mut wallet_spent: i64 = 0;
    if crate::routes::admin_util::checkbox(&form.use_wallet) && user.wallet_balance > 0 {
        let remaining = (subtotal - discount.amount).max(0);
        wallet_spent = user.wallet_balance.min(remaining);
        if wallet_spent > 0 {
            discount.amount += wallet_spent;
        }
    }

    let order = db::orders::create_from_cart(
        &s.db,
        Some(user.id),
        &items,
        &ship,
        method,
        ship_method.rial,
        Some(ship_method.label.as_str()),
        &discount,
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Protocol(msg) => AppError::Domain(domain::DomainError::OutOfStock(msg)),
        other => AppError::Db(other),
    })?;

    // یادداشت سفارش (در صورت وجود).
    if let Some(note) = form.note.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        let _ = db::orders::set_note(&s.db, order.id, note).await;
    }

    // کسر امتیاز خرج‌شده (پس از ساخت موفق سفارش).
    if points_spent > 0 {
        if let Err(e) = db::loyalty::add_transaction(
            &s.db,
            user.id,
            -points_spent,
            "redeem",
            Some(order.id),
            Some("استفاده از امتیاز در خرید"),
        )
        .await
        {
            tracing::error!(error = %e, "کسر امتیاز خرج‌شده شکست خورد");
        }
    }

    // کسر مبلغ کیف پول (پس از ساخت موفق سفارش).
    if wallet_spent > 0 {
        if let Err(e) = db::wallet::add_transaction(
            &s.db,
            user.id,
            -wallet_spent,
            "spend",
            Some(order.id),
            Some("پرداخت از کیف پول در خرید"),
        )
        .await
        {
            tracing::error!(error = %e, "کسر مبلغ کیف پول شکست خورد");
        }
    }

    // ثبت استفاده‌ی کوپن (پس از ساخت موفق سفارش).
    if let Some(coupon) = &applied_coupon {
        if let Err(e) =
            db::coupons::record_redemption(&s.db, coupon.id, Some(user.id), order.id, discount.amount)
                .await
        {
            tracing::error!(error = %e, "ثبت استفاده‌ی کوپن شکست خورد");
        }
    }

    // سبد را خالی می‌کنیم.
    db::cart::clear(&s.db, cart_id).await?;

    if method == "cod" {
        // پرداخت در محل: امتیاز خرید را همین‌جا اعطا و در صورت اتصال، در تلگرام اطلاع می‌دهیم.
        crate::loyalty_hook::award_and_notify(&s, &user, &order).await;
        // پرداخت در محل: مستقیم به صفحه‌ی نتیجه.
        return Ok(Redirect::to(&format!("/order/{}", order.id)).into_response());
    }

    // پرداخت آنلاین: ساخت رکورد پرداخت و ریدایرکت به درگاه.
    if !s.settings.payments_enabled() {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "درگاه پرداخت آنلاین هنوز فعال نشده است. لطفاً پرداخت در محل (زنجان) را انتخاب کنید یا با پشتیبانی تماس بگیرید.".into(),
        )));
    }

    let Some(gateway) = payments::build(&s.payment.provider, &s.payment.merchant) else {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "درگاه پرداخت پیکربندی نشده است.".into(),
        )));
    };

    let payment = db::orders::create_payment(&s.db, order.id, gateway.name(), order.total).await?;
    let callback = format!("{}/payment/callback?provider={}", s.base_url, gateway.name());
    let req = payments::PayRequest {
        amount_rial: order.total as u64,
        order_id: order.order_number.to_string(),
        callback_url: callback,
        description: format!("سفارش #{} زاگرس", order.order_number),
        mobile: Some(user.mobile.clone()),
    };

    match gateway.request(&req).await {
        Ok(init) => {
            db::orders::set_payment_token(&s.db, payment.id, &init.gateway_token).await?;
            Ok(Redirect::to(&init.redirect_url).into_response())
        }
        Err(e) => {
            tracing::error!(error = %e, "شروع پرداخت ناموفق");
            Err(AppError::Domain(domain::DomainError::Invalid(
                "خطا در اتصال به درگاه پرداخت. دوباره تلاش کنید.".into(),
            )))
        }
    }
}

// ---------- callback پرداخت ----------

#[derive(Deserialize)]
pub struct CallbackParams {
    pub provider: String,
    // زرین‌پال: Authority + Status ؛ زیبال: trackId + success + status
    #[serde(alias = "Authority")]
    pub authority: Option<String>,
    #[serde(alias = "Status")]
    pub status: Option<String>,
    #[serde(alias = "trackId")]
    pub track_id: Option<String>,
    pub success: Option<String>,
}

pub async fn callback(
    State(s): State<AppState>,
    Query(p): Query<CallbackParams>,
) -> AppResult<axum::response::Response> {
    let token = p.authority.or(p.track_id).unwrap_or_default();
    if token.is_empty() {
        return Ok(Redirect::to("/payment/failed").into_response());
    }

    // یافتن رکورد پرداخت با توکن (لینک ما به سفارش).
    let Some(payment) = db::orders::find_payment_by_token(&s.db, &p.provider, &token).await? else {
        return Ok(Redirect::to("/payment/failed").into_response());
    };

    // اگر قبلاً تأیید شده (callback تکراری)، مستقیم به سفارش.
    if payment.state == "verified" {
        return Ok(Redirect::to(&format!("/order/{}", payment.order_id)).into_response());
    }

    // اگر کاربر لغو کرد (Status=NOK یا success=0) — موجودی رزروشده آزاد می‌شود.
    let cancelled = p.status.as_deref() == Some("NOK") || p.success.as_deref() == Some("0");
    if cancelled {
        db::orders::cancel_and_restock(&s.db, payment.order_id).await?;
        return Ok(Redirect::to("/payment/failed").into_response());
    }

    let Some(gateway) = payments::build(&p.provider, &s.payment.merchant) else {
        return Ok(Redirect::to("/payment/failed").into_response());
    };

    match gateway.verify(&token, payment.amount as u64).await {
        Ok(result) if result.paid => {
            db::orders::mark_payment_verified(
                &s.db,
                payment.id,
                &result.ref_id,
                result.card_pan.as_deref(),
            )
            .await?;
            // اعطای امتیاز خرید + اطلاع‌رسانی تلگرام (idempotent در برابر callback تکراری).
            if let Ok(Some(order)) = db::orders::find(&s.db, payment.order_id).await {
                if let Some(uid) = order.user_id {
                    if let Ok(Some(user)) = db::accounts::find_by_id(&s.db, uid).await {
                        crate::loyalty_hook::award_and_notify(&s, &user, &order).await;
                    }
                }
            }
            Ok(Redirect::to(&format!("/order/{}", payment.order_id)).into_response())
        }
        _ => {
            db::orders::cancel_and_restock(&s.db, payment.order_id).await?;
            Ok(Redirect::to("/payment/failed").into_response())
        }
    }
}

// ---------- صفحه‌ی نتیجه‌ی سفارش ----------

#[derive(Template)]
#[template(path = "order_result.html")]
struct OrderResultTemplate {
    site_name: String,
    order: db::orders::Order,
    order_number_fa: String,
    items: Vec<db::orders::OrderLine>,
    paid: bool,
    is_cod: bool,
    can_cancel: bool,
    cancelled: bool,
    can_return: bool,
    timeline: Vec<TimelineStep>,
}

struct TimelineStep {
    label: &'static str,
    state: &'static str, // done | current | todo
}

/// خط زمانی وضعیت سفارش بر اساس روش پرداخت و وضعیت فعلی.
fn build_timeline(status: &str, is_cod: bool) -> Vec<TimelineStep> {
    let steps: &[(&str, &str)] = if is_cod {
        &[
            ("cod_pending", "ثبت سفارش"),
            ("processing", "در حال پردازش"),
            ("shipped", "ارسال"),
            ("delivered", "تحویل"),
        ]
    } else {
        &[
            ("awaiting_payment", "ثبت سفارش"),
            ("paid", "پرداخت"),
            ("processing", "در حال پردازش"),
            ("shipped", "ارسال"),
            ("delivered", "تحویل"),
        ]
    };
    let cur_idx = steps.iter().position(|(k, _)| *k == status);
    steps
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            let state = match cur_idx {
                Some(ci) if i < ci => "done",
                Some(ci) if i == ci => "current",
                _ => "todo",
            };
            TimelineStep { label, state }
        })
        .collect()
}

pub async fn order_result(
    State(s): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(order) = db::orders::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("سفارش".into())));
    };
    // فقط صاحب سفارش ببیند.
    if let (Some(u), Some(ou)) = (&user, order.user_id) {
        if u.id != ou {
            return Err(AppError::Domain(domain::DomainError::Unauthorized));
        }
    }

    let items = db::orders::items_for(&s.db, order.id).await?;
    let order_number_fa = crate::filters::to_fa_digits(&order.order_number.to_string());
    let paid = order.status == "paid";
    let is_cod = order.payment_method == "cod";
    // مشتری می‌تواند سفارش پرداخت‌نشده/در انتظار را لغو کند.
    let can_cancel = matches!(order.status.as_str(), "awaiting_payment" | "pending" | "cod_pending");
    let cancelled = order.status == "cancelled";
    // مرجوعی فقط برای سفارش تحویل‌شده و در صورت نبود مرجوعی باز.
    let can_return = order.status == "delivered"
        && !db::returns::exists_open_for_order(&s.db, order.id).await.unwrap_or(true);
    let timeline = build_timeline(&order.status, is_cod);
    render(OrderResultTemplate {
        site_name: s.site_name(),
        order,
        order_number_fa,
        items,
        paid,
        is_cod,
        can_cancel,
        cancelled,
        can_return,
        timeline,
    })
    .map(|h| h.into_response())
}

// ---------- فاکتور قابل‌چاپ ----------

#[derive(Template)]
#[template(path = "invoice.html")]
struct InvoiceTemplate {
    site_name: String,
    contact_phone: String,
    contact_address: String,
    order: db::orders::Order,
    order_number_fa: String,
    items: Vec<db::orders::OrderLine>,
    date_fa: String,
}

pub async fn invoice(
    State(s): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    CurrentUser(user): CurrentUser,
) -> AppResult<axum::response::Response> {
    let Some(order) = db::orders::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("سفارش".into())));
    };
    // فقط صاحب سفارش یا مدیر.
    let allowed = match &user {
        Some(u) => order.user_id == Some(u.id) || u.role == "admin",
        None => false,
    };
    if !allowed {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    let items = db::orders::items_for(&s.db, order.id).await?;
    let order_number_fa = crate::filters::to_fa_digits(&order.order_number.to_string());
    let date_fa = crate::filters::to_fa_digits(&order.created_at.format("%Y-%m-%d %H:%M").to_string());
    render(InvoiceTemplate {
        site_name: s.site_name(),
        contact_phone: s.settings.contact_phone(),
        contact_address: s.settings.contact_address(),
        order,
        order_number_fa,
        items,
        date_fa,
    })
    .map(|h| h.into_response())
}

/// سفارش مجدد: اقلام یک سفارش قبلی را به سبد اضافه می‌کند.
pub async fn reorder(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(order) = db::orders::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("سفارش".into())));
    };
    if order.user_id != Some(user.id) {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    let (jar, key) = crate::routes::cart::ensure_cart_cookie(jar);
    let cart_id = db::cart::get_or_create_cart(&s.db, &key).await?;
    db::orders::reorder_into_cart(&s.db, order.id, cart_id).await?;
    Ok((jar, Redirect::to("/cart")).into_response())
}

/// لغو سفارش توسط مشتری (فقط سفارش‌های پرداخت‌نشده/در انتظار).
pub async fn cancel(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> AppResult<axum::response::Response> {
    let Some(user) = user else {
        return Ok(Redirect::to("/account/login").into_response());
    };
    let Some(order) = db::orders::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound("سفارش".into())));
    };
    if order.user_id != Some(user.id) {
        return Err(AppError::Domain(domain::DomainError::Unauthorized));
    }
    if !matches!(order.status.as_str(), "awaiting_payment" | "pending" | "cod_pending") {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "این سفارش در وضعیتی نیست که قابل لغو باشد.".into(),
        )));
    }
    if db::orders::cancel_and_restock(&s.db, order.id).await? {
        let _ = db::loyalty::reverse_purchase_points(&s.db, order.id).await;
        // مبلغ کیف پولِ خرج‌شده روی این سفارش به کاربر بازگردانده می‌شود.
        let _ = db::wallet::reverse_order_spend(&s.db, order.id).await;
    }
    Ok(Redirect::to(&format!("/order/{}", order.id)).into_response())
}

#[derive(Template)]
#[template(path = "payment_failed.html")]
struct FailedTemplate {
    site_name: String,
}

pub async fn failed(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(FailedTemplate {
        site_name: s.site_name(),
    })
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

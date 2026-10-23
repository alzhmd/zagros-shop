pub mod account;
pub mod admin;
pub mod admin_banners;
pub mod admin_coupons;
pub mod admin_extra;
pub mod admin_orders;
pub mod admin_products;
pub mod admin_qa;
pub mod admin_returns;
pub mod admin_reviews;
pub mod admin_settings;
pub mod admin_taxonomy;
pub mod admin_users;
pub mod admin_util;
pub mod cart;
pub mod catalog;
pub mod checkout;
pub mod home;
pub mod offers;
pub mod pages;
pub mod partials;
pub mod repair;
pub mod returns;
pub mod reviews;
pub mod seo;
pub mod support;
pub mod wishlist;

use crate::state::AppState;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};

pub fn router(state: AppState) -> Router {
    // asset استاتیک با کش طولانی (یک سال، immutable) — فایل‌ها نسخه‌دارند/کم‌تغییر.
    let assets = ServeDir::new("assets");
    let assets_service = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ))
        .service(assets);

    Router::new()
        .route("/", get(home::index))
        // پاسخ‌های جزئی HTMX (هدر/فوتر/جستجو).
        .route("/partials/header", get(partials::header))
        .route("/partials/cart-mini", get(partials::cart_mini))
        .route("/partials/cart-mini-badge", get(partials::cart_mini_badge))
        .route("/partials/menu", get(partials::menu))
        .route("/partials/footer", get(partials::footer))
        .route("/partials/search", get(partials::search))
        .route("/products", get(catalog::index))
        .route("/categories", get(catalog::categories_page))
        .route("/product/{slug}", get(catalog::product))
        .route("/offers", get(offers::index))
        .route("/brand/{slug}", get(catalog::brand_landing))
        .route("/cart", get(cart::view))
        .route("/cart/add", post(cart::add))
        .route("/cart/update", post(cart::update))
        .route("/cart/remove", post(cart::remove))
        .route("/cart/save", post(cart::save_for_later))
        .route("/cart/move-to-cart", post(cart::move_to_cart))
        .route("/checkout", get(checkout::page).post(checkout::submit))
        .route("/checkout/coupon", post(checkout::apply_coupon))
        .route("/payment/callback", get(checkout::callback))
        .route("/payment/failed", get(checkout::failed))
        .route("/order/{id}", get(checkout::order_result))
        .route("/order/{id}/invoice", get(checkout::invoice))
        .route("/order/{id}/reorder", post(checkout::reorder))
        .route("/order/{id}/cancel", post(checkout::cancel))
        .route(
            "/order/{id}/return",
            get(returns::request_form).post(returns::submit),
        )
        .route("/account/returns", get(returns::list))
        .route("/return/{number}", get(returns::detail))
        .route("/return/cancel", post(returns::cancel))
        .route("/account", get(account::dashboard))
        .route("/account/orders", get(account::orders))
        .route("/account/repairs", get(account::repairs))
        .route(
            "/account/login",
            get(account::login_page).post(account::send_otp),
        )
        .route("/account/verify", post(account::verify_otp))
        .route("/account/logout", post(account::logout))
        .route("/account/club", get(account::club))
        .route("/account/wallet", get(account::wallet))
        .route("/account/notifications", get(account::notifications))
        .route("/alerts/stock", post(account::subscribe_stock))
        .route("/account/club/link", post(account::generate_link_code))
        .route("/account/wishlist", get(wishlist::page))
        .route(
            "/account/profile",
            get(account::profile_page).post(account::update_profile),
        )
        .route("/account/delete", post(account::delete_account))
        .route("/account/wallet/redeem-gift", post(account::redeem_gift))
        .route(
            "/account/addresses",
            get(account::addresses_page).post(account::add_address),
        )
        .route(
            "/account/addresses/{id}/default",
            post(account::set_default_address),
        )
        .route(
            "/account/addresses/{id}/delete",
            post(account::delete_address),
        )
        .route("/account/reviews", get(account::reviews))
        .route("/account/sessions", get(account::sessions))
        .route("/account/sessions/revoke", post(account::revoke_session))
        .route(
            "/account/sessions/revoke-others",
            post(account::revoke_other_sessions),
        )
        .route("/wishlist/toggle", post(wishlist::toggle))
        .route("/reviews", post(reviews::submit))
        .route("/reviews/{id}/helpful", post(reviews::helpful))
        .route("/questions", post(reviews::ask))
        .route("/repair", get(repair::form).post(repair::submit))
        .route(
            "/repair/track",
            get(repair::lookup_page).post(repair::lookup),
        )
        .route("/repair/track/{ticket}", get(repair::track))
        // پشتیبانی: تیکت
        .route("/support", get(support::hub))
        .route("/support/ticket", post(support::create_ticket))
        .route("/support/ticket-lookup", post(support::ticket_lookup))
        .route("/support/ticket/{number}", get(support::ticket_thread))
        .route(
            "/support/ticket/{number}/reply",
            post(support::ticket_reply),
        )
        // پشتیبانی: چت زنده
        .route(
            "/support/chat",
            get(support::chat_start_page).post(support::chat_start),
        )
        .route("/support/chat/{id}", get(support::chat_room))
        .route("/support/chat/{id}/ws", get(support::chat_ws))
        // پنل ادمین
        .route("/admin", get(admin::dashboard))
        .route("/admin/repairs", get(admin::repairs))
        .route(
            "/admin/repairs/{id}",
            get(admin::repair_detail).post(admin::update_repair),
        )
        .route(
            "/admin/repairs/{id}/details",
            post(admin::update_repair_details),
        )
        .route("/admin/orders", get(admin_orders::list))
        .route(
            "/admin/orders/{id}",
            get(admin_orders::detail).post(admin_orders::update_status),
        )
        // پنل ادمین: کاربران
        .route("/admin/users", get(admin_users::list))
        .route("/admin/users/{id}", get(admin_users::detail))
        .route("/admin/users/{id}/active", post(admin_users::toggle_active))
        .route("/admin/users/{id}/role", post(admin_users::set_role))
        .route("/admin/users/{id}/points", post(admin_users::adjust_points))
        .route("/admin/users/{id}/wallet", post(admin_users::adjust_wallet))
        .route("/admin/tickets", get(admin::tickets))
        .route("/admin/tickets/{id}", get(admin::ticket_detail))
        .route("/admin/tickets/{id}/reply", post(admin::ticket_reply))
        .route("/admin/tickets/{id}/status", post(admin::ticket_status))
        .route("/admin/chat", get(admin::chats))
        .route("/admin/chat/{id}", get(admin::chat_room))
        .route("/admin/chat/{id}/close", post(admin::close_chat))
        // پنل ادمین: مدیریت محصول
        .route(
            "/admin/products",
            get(admin_products::list).post(admin_products::create),
        )
        .route("/admin/products/new", get(admin_products::new_form))
        .route("/admin/products/{id}/edit", get(admin_products::edit_form))
        .route("/admin/products/{id}", post(admin_products::update))
        .route("/admin/products/{id}/delete", post(admin_products::delete))
        .route(
            "/admin/products/{id}/active",
            post(admin_products::toggle_active),
        )
        .route(
            "/admin/products/{id}/featured",
            post(admin_products::toggle_featured),
        )
        .route(
            "/admin/products/{id}/variants",
            post(admin_products::add_variant),
        )
        .route(
            "/admin/products/{id}/variants/{vid}",
            post(admin_products::update_variant),
        )
        .route(
            "/admin/products/{id}/variants/{vid}/delete",
            post(admin_products::delete_variant),
        )
        .route(
            "/admin/products/{id}/images",
            post(admin_products::add_image)
                // آپلود تصویر تا ۸ مگابایت (پیش‌فرض axum فقط ۲ مگابایت است).
                .layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/admin/products/{id}/images/{iid}/delete",
            post(admin_products::delete_image),
        )
        .route(
            "/admin/products/{id}/images/{iid}/primary",
            post(admin_products::set_primary_image),
        )
        .route("/admin/products/{id}/attrs", post(admin_products::set_attr))
        .route(
            "/admin/products/{id}/attrs/new",
            post(admin_products::create_attr),
        )
        // پنل ادمین: دسته‌بندی
        .route(
            "/admin/categories",
            get(admin_taxonomy::categories_list).post(admin_taxonomy::category_create),
        )
        .route("/admin/categories/new", get(admin_taxonomy::category_new))
        .route(
            "/admin/categories/{id}/edit",
            get(admin_taxonomy::category_edit),
        )
        .route(
            "/admin/categories/{id}",
            post(admin_taxonomy::category_update),
        )
        .route(
            "/admin/categories/{id}/delete",
            post(admin_taxonomy::category_delete),
        )
        .route(
            "/admin/categories/{id}/active",
            post(admin_taxonomy::category_toggle_active),
        )
        // پنل ادمین: برند
        .route(
            "/admin/brands",
            get(admin_taxonomy::brands_list).post(admin_taxonomy::brand_create),
        )
        .route("/admin/brands/new", get(admin_taxonomy::brand_new))
        .route("/admin/brands/{id}/edit", get(admin_taxonomy::brand_edit))
        .route("/admin/brands/{id}", post(admin_taxonomy::brand_update))
        .route(
            "/admin/brands/{id}/delete",
            post(admin_taxonomy::brand_delete),
        )
        // پنل ادمین: کد تخفیف
        .route(
            "/admin/coupons",
            get(admin_coupons::list).post(admin_coupons::create),
        )
        .route("/admin/coupons/new", get(admin_coupons::new_form))
        .route("/admin/coupons/{id}/edit", get(admin_coupons::edit_form))
        .route("/admin/coupons/{id}", post(admin_coupons::update))
        .route(
            "/admin/coupons/{id}/active",
            post(admin_coupons::toggle_active),
        )
        .route("/admin/coupons/{id}/delete", post(admin_coupons::delete))
        // پنل ادمین: مرجوعی
        .route("/admin/returns", get(admin_returns::list))
        .route(
            "/admin/returns/{id}",
            get(admin_returns::detail).post(admin_returns::update_status),
        )
        // پنل ادمین: نظرات
        .route("/admin/reviews", get(admin_reviews::list))
        .route(
            "/admin/reviews/{id}/status",
            post(admin_reviews::set_status),
        )
        .route("/admin/reviews/{id}/delete", post(admin_reviews::delete))
        // پنل ادمین: پرسش و پاسخ
        .route("/admin/questions", get(admin_qa::list))
        .route("/admin/questions/{id}/answer", post(admin_qa::answer))
        .route("/admin/questions/{id}/status", post(admin_qa::set_status))
        .route("/admin/questions/{id}/delete", post(admin_qa::delete))
        // پنل ادمین: بنرها
        .route(
            "/admin/banners",
            get(admin_banners::list).post(admin_banners::create),
        )
        .route("/admin/banners/new", get(admin_banners::new_form))
        .route("/admin/banners/{id}/edit", get(admin_banners::edit_form))
        .route("/admin/banners/{id}", post(admin_banners::update))
        .route("/admin/banners/{id}/delete", post(admin_banners::delete))
        // پنل ادمین: کارت هدیه و اعلان همگانی
        .route(
            "/admin/giftcards",
            get(admin_extra::giftcards).post(admin_extra::create_giftcard),
        )
        .route(
            "/admin/notify",
            get(admin_extra::notify_page).post(admin_extra::notify_send),
        )
        // پنل ادمین: تنظیمات سایت
        .route(
            "/admin/settings",
            get(admin_settings::page_with_flash).post(admin_settings::save),
        )
        .route("/about", get(pages::about))
        .route("/contact", get(pages::contact))
        .route("/faq", get(pages::faq))
        .route("/terms", get(pages::terms))
        .route("/robots.txt", get(seo::robots))
        .route("/sitemap.xml", get(seo::sitemap))
        .route("/health", get(health))
        .nest_service("/assets", assets_service)
        // محافظت CSRF (بررسی Origin/Referer روی POST/PUT/PATCH/DELETE) — پیش از فشرده‌سازی.
        .layer(axum::middleware::from_fn(crate::csrf::guard))
        // فشرده‌سازی پاسخ (brotli/gzip) — درخواست‌های WebSocket (upgrade) دست‌نخورده می‌مانند.
        .layer(CompressionLayer::new())
        // سرآیندهای امنیتی پایه روی همه‌ی پاسخ‌ها.
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        // سیاست امنیت محتوا (CSP): همه‌چیز خودمیزبان. اسکریپت/استایل اینلاین (Alpine، JSON-LD،
        // اسکریپت‌های کوچک تب‌فعال) مجازند؛ ویدئوی محصول از یوتیوب/آپارات در iframe.
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; \
                 img-src 'self' data: https:; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                 style-src 'self' 'unsafe-inline'; \
                 font-src 'self'; \
                 connect-src 'self' ws: wss:; \
                 frame-src https://www.youtube.com https://www.aparat.com; \
                 base-uri 'self'; \
                 form-action 'self'",
            ),
        ))
        // ضدِ کش برای پاسخ‌های پویا: ناوبریِ وابسته به ورود (لینکِ پنل مدیریت، نام کاربر،
        // شمارِ سبد) نباید در مرورگر کش شود؛ وگرنه پس از ورود/خروج نسخه‌ی کهنه نشان داده می‌شود.
        // asset‌ها cache-control اختصاصی (immutable) دارند، پس if_not_present دست‌نخورده ردشان می‌کند.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

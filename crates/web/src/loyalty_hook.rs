//! قلاب باشگاه مشتریان: اعطای امتیاز خرید + اطلاع‌رسانی تلگرام (فاز ۵).
//! از دو مسیر فراخوانی می‌شود: پرداخت در محل (هنگام ثبت) و پرداخت آنلاین (هنگام تأیید).

use crate::state::AppState;
use db::accounts::User;
use db::orders::Order;

/// امتیاز خرید را به‌صورت idempotent اعطا می‌کند و در صورت اتصال تلگرام، اطلاع می‌دهد.
/// هر خطا فقط لاگ می‌شود تا جریان سفارش/پرداخت نشکند.
pub async fn award_and_notify(state: &AppState, user: &User, order: &Order) {
    let rate = state.settings.rial_per_point();
    let points = match db::loyalty::award_purchase_points(
        &state.db,
        user.id,
        order.id,
        order.total,
        rate,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, order = %order.order_number, "اعطای امتیاز خرید شکست خورد");
            0
        }
    };

    // اعلان درون‌سایتی تأیید سفارش.
    {
        let num = crate::filters::to_fa_digits(&order.order_number.to_string());
        let title = format!("سفارش #{num} ثبت شد");
        let body = format!("مبلغ: {}", domain::Rial(order.total).format_toman_fa());
        let url = format!("/order/{}", order.id);
        let _ = db::notifications::create(&state.db, user.id, "order", &title, Some(&body), Some(&url)).await;
    }

    // اطلاع‌رسانی تلگرام (فقط اگر ربات فعال و حساب متصل باشد).
    if let (Some(bot), Some(tg_id)) = (&state.bot, user.telegram_id) {
        let total_fa = domain::Rial(order.total).format_toman_fa();
        let mut msg = format!(
            "🧾 سفارش شما با شماره #{} ثبت شد.\nمبلغ: {}",
            order.order_number, total_fa
        );
        if points > 0 {
            msg.push_str(&format!("\n💎 {points} امتیاز باشگاه دریافت کردید."));
        }
        bot.notify(tg_id, &msg).await;
    }

    // ایمیل تأیید سفارش (اگر کاربر ایمیل داشته باشد).
    if let Some(email) = user.email.as_deref().filter(|e| e.contains('@')) {
        let total_fa = domain::Rial(order.total).format_toman_fa();
        let body = format!(
            "سلام،\n\nسفارش شما با شماره #{} با موفقیت ثبت شد.\nمبلغ کل: {}\n\nوضعیت سفارش را می‌توانید از حساب کاربری‌تان در زاگرس پیگیری کنید.\n\nبا احترام،\nزاگرس — جهان کولاک",
            order.order_number, total_fa
        );
        state
            .mailer
            .send(email, &format!("تأیید سفارش #{}", order.order_number), &body)
            .await;
    }
}

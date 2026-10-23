//! پردازش دستورهای ربات تلگرام و تولید پاسخ فارسی.

use db::accounts::{self, LinkOutcome};
use db::loyalty;
use db::Db;

/// متن خوش‌آمد و راهنما.
const HELP: &str = "🧊 ربات «زاگرس — جهان کولاک»\n\n\
دستورها:\n\
/points — نمایش امتیاز باشگاه مشتریان شما\n\
/help — همین راهنما\n\n\
برای اتصال حسابتان، از صفحه‌ی حساب کاربری در سایت کد اتصال بگیرید و دستور زیر را بزنید:\n\
/start کد‌شما";

/// یک پیام ورودی را پردازش می‌کند و متن پاسخ را برمی‌گرداند (در صورت نیاز به پاسخ).
pub async fn handle_message(db: &Db, telegram_id: i64, text: &str) -> Option<String> {
    let text = text.trim();

    // /start ممکن است پارامتر اتصال داشته باشد: «/start <code>».
    if let Some(rest) = strip_command(text, "start") {
        let code = rest.trim();
        if code.is_empty() {
            return Some(welcome_text(db, telegram_id).await);
        }
        return Some(link_account(db, telegram_id, code).await);
    }

    if strip_command(text, "points").is_some() {
        return Some(points_text(db, telegram_id).await);
    }

    if strip_command(text, "help").is_some() {
        return Some(HELP.to_string());
    }

    // پیام ناشناخته → راهنما.
    Some(HELP.to_string())
}

/// اگر متن با دستور `/name` شروع شود، باقی متن (بعد از دستور) را برمی‌گرداند.
/// از فرم گروهی `/name@botusername` هم پشتیبانی می‌کند.
fn strip_command<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let first = text.split_whitespace().next()?;
    let cmd = first.strip_prefix('/')?;
    // حذف بخش @botusername در صورت وجود.
    let cmd = cmd.split('@').next().unwrap_or(cmd);
    if cmd.eq_ignore_ascii_case(name) {
        Some(text[first.len()..].trim_start())
    } else {
        None
    }
}

async fn welcome_text(db: &Db, telegram_id: i64) -> String {
    match accounts::find_by_telegram_id(db, telegram_id).await {
        Ok(Some(_)) => {
            format!("سلام! حساب شما به ربات متصل است. ✅\n\nبرای دیدن امتیاز /points را بزنید.")
        }
        _ => HELP.to_string(),
    }
}

async fn link_account(db: &Db, telegram_id: i64, code: &str) -> String {
    match accounts::consume_link_code(db, code, telegram_id).await {
        Ok(LinkOutcome::Linked(user)) => {
            // پاداش اتصال — فقط بار اول (اگر قبلاً تراکنش تلگرام نداشته باشد، اعطا می‌شود).
            let bonus = grant_link_bonus(db, &user).await;
            let mut msg = String::from("🎉 حساب شما با موفقیت به ربات متصل شد!\n");
            if bonus > 0 {
                msg.push_str(&format!("{bonus} امتیاز پاداش اتصال دریافت کردید.\n"));
            }
            msg.push_str("\nبرای دیدن امتیاز /points را بزنید.");
            msg
        }
        Ok(LinkOutcome::AlreadyLinkedToOther) => {
            "این حساب تلگرام قبلاً به کاربر دیگری متصل شده است.".to_string()
        }
        Ok(LinkOutcome::InvalidOrExpired) => {
            "کد اتصال نامعتبر یا منقضی شده است. لطفاً از سایت کد تازه بگیرید.".to_string()
        }
        Err(e) => {
            tracing::error!(error = %e, "اتصال حساب تلگرام شکست خورد");
            "خطایی رخ داد. کمی بعد دوباره تلاش کنید.".to_string()
        }
    }
}

/// اعطای پاداش یک‌باره‌ی اتصال تلگرام؛ اگر قبلاً اعطا شده باشد ۰ برمی‌گرداند.
async fn grant_link_bonus(db: &Db, user: &accounts::User) -> i64 {
    // اگر کاربر قبلاً تراکنش «telegram» داشته، دوباره نده.
    match loyalty::has_reason(db, user.id, "telegram").await {
        Ok(true) | Err(_) => 0,
        Ok(false) => {
            let bonus = loyalty::TELEGRAM_LINK_BONUS;
            if let Err(e) =
                loyalty::add_transaction(db, user.id, bonus, "telegram", None, Some("اتصال ربات"))
                    .await
            {
                tracing::error!(error = %e, "اعطای پاداش تلگرام شکست خورد");
                return 0;
            }
            bonus
        }
    }
}

async fn points_text(db: &Db, telegram_id: i64) -> String {
    match accounts::find_by_telegram_id(db, telegram_id).await {
        Ok(Some(user)) => {
            let history = loyalty::history(db, user.id, 5).await.unwrap_or_default();
            let mut msg = format!("💎 امتیاز باشگاه مشتریان شما: {}\n", user.points);
            if !history.is_empty() {
                msg.push_str("\nآخرین تراکنش‌ها:\n");
                for tx in history {
                    msg.push_str(&format!("• {} {}\n", tx.delta_signed(), tx.reason_fa()));
                }
            }
            msg
        }
        Ok(None) => "حساب شما هنوز به ربات متصل نیست.\nاز صفحه‌ی حساب کاربری در سایت، کد اتصال بگیرید و با /start کد بفرستید.".to_string(),
        Err(e) => {
            tracing::error!(error = %e, "خواندن امتیاز شکست خورد");
            "خطایی رخ داد. کمی بعد دوباره تلاش کنید.".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::strip_command;

    #[test]
    fn plain_command() {
        assert_eq!(strip_command("/help", "help"), Some(""));
        assert_eq!(strip_command("/points", "points"), Some(""));
    }

    #[test]
    fn command_with_argument() {
        // «/start <code>» باید کد را به‌عنوان باقی‌مانده برگرداند.
        assert_eq!(strip_command("/start 9NWZS74A", "start"), Some("9NWZS74A"));
        assert_eq!(strip_command("/start   ABC123", "start"), Some("ABC123"));
    }

    #[test]
    fn group_mention_form() {
        // فرم گروهی «/start@botname» باید پشتیبانی شود.
        assert_eq!(
            strip_command("/start@znjzagrosbot CODE", "start"),
            Some("CODE")
        );
        assert_eq!(strip_command("/help@znjzagrosbot", "help"), Some(""));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(strip_command("/START code", "start"), Some("code"));
    }

    #[test]
    fn non_matching() {
        assert_eq!(strip_command("/points", "start"), None);
        assert_eq!(strip_command("hello", "start"), None);
        assert_eq!(strip_command("", "start"), None);
        assert_eq!(strip_command("start without slash", "start"), None);
    }
}

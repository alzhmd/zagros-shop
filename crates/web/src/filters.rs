//! فیلترهای askama و کمک‌توابع نمایش.
//! askama فیلترهای سفارشی را از ماژولی به نام `filters` در ریشه‌ی crate پیدا می‌کند.

/// نمایش قیمت ریالی به «۱٬۲۵۰٬۰۰۰ تومان».
pub fn toman_string(rial: i64) -> String {
    domain::Rial(rial).format_toman_fa()
}

/// تبدیل ارقام لاتین رشته به فارسی.
pub fn to_fa_digits(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_digit() {
                char::from_u32('۰' as u32 + (c as u32 - '0' as u32)).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

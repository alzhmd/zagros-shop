//! کمک‌توابع مشترک فرم‌های پنل ادمین — اعتبارسنجی نامک، چک‌باکس، اعداد.

use uuid::Uuid;

/// نرمال‌سازی و اعتبارسنجی slug: حروف کوچک، فقط [a-z0-9-].
/// نام‌های فارسی خالص → رشته‌ی خالی (ادمین باید نامک انگلیسی بدهد).
pub fn clean_slug(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut prev_dash = false;
    for ch in lowered.chars() {
        let c = if ch == ' ' || ch == '_' { '-' } else { ch };
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if c == '-' && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// چک‌باکس HTML: حاضر بودن مقدار = true.
pub fn checkbox(v: &Option<String>) -> bool {
    matches!(v.as_deref(), Some(s) if s == "on" || s == "true" || s == "1")
}

/// تجزیه‌ی UUID اختیاری از ورودی فرم (خالی → None).
pub fn parse_uuid_opt(v: &Option<String>) -> Option<Uuid> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// تجزیه‌ی عدد از رشته (با تحمل ارقام فارسی و جداکننده‌ها مثل ٬ و ,).
pub fn parse_digits(s: &str) -> Result<i64, String> {
    let normalized: String = s
        .chars()
        .filter_map(|c| {
            if c.is_ascii_digit() {
                Some(c)
            } else if ('۰'..='۹').contains(&c) {
                char::from_u32('0' as u32 + (c as u32 - '۰' as u32))
            } else {
                None // حذف جداکننده‌ها و فاصله
            }
        })
        .collect();
    if normalized.is_empty() {
        return Err("عدد نامعتبر است.".into());
    }
    normalized.parse::<i64>().map_err(|_| "عدد خیلی بزرگ است.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_cleaning() {
        assert_eq!(clean_slug("Emersun TF-23"), "emersun-tf-23");
        assert_eq!(clean_slug("  Hot & Cold  "), "hot-cold");
        assert_eq!(clean_slug("a___b   c"), "a-b-c");
        assert_eq!(clean_slug("--trim--"), "trim");
        // نام فارسی خالص → خالی (ادمین باید نامک انگلیسی بدهد).
        assert_eq!(clean_slug("یخچال"), "");
    }

    #[test]
    fn digit_parsing() {
        assert_eq!(parse_digits("۱۴۲۰۰۰۰۰").unwrap(), 14200000);
        assert_eq!(parse_digits("5,000,000").unwrap(), 5000000);
        assert_eq!(parse_digits("۱٬۲۵۰").unwrap(), 1250);
        assert!(parse_digits("abc").is_err());
    }

    #[test]
    fn checkbox_truthiness() {
        assert!(checkbox(&Some("on".into())));
        assert!(!checkbox(&None));
        assert!(!checkbox(&Some("".into())));
    }
}

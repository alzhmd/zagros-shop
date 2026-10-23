use serde::{Deserialize, Serialize};
use std::fmt;

/// مبلغ به **ریال** — واحد داخلی سیستم. هرگز از f64 برای پول استفاده نمی‌کنیم.
/// نمایش در UI به تومان (تقسیم بر ۱۰) انجام می‌شود.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rial(pub i64);

impl Rial {
    pub const ZERO: Rial = Rial(0);

    #[inline]
    pub fn from_toman(toman: i64) -> Self {
        Rial(toman * 10)
    }

    #[inline]
    pub fn to_toman(self) -> i64 {
        self.0 / 10
    }

    /// قالب‌بندی فارسی با جداکننده‌ی هزارگان، به تومان. مثال: «۱٬۲۵۰٬۰۰۰ تومان»
    pub fn format_toman_fa(self) -> String {
        let toman = self.to_toman();
        format!("{} تومان", group_digits_fa(toman))
    }
}

impl fmt::Display for Rial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// تبدیل ارقام لاتین یک رشته به فارسی (سایر کاراکترها دست‌نخورده).
pub fn fa_digits(s: &str) -> String {
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

/// جداکننده‌ی هزارگان با ارقام فارسی (٬ = جداکننده‌ی فارسی).
fn group_digits_fa(n: i64) -> String {
    let neg = n < 0;
    let digits: Vec<char> = n.abs().to_string().chars().collect();
    let mut out = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('٬');
        }
        // تبدیل رقم لاتین به فارسی
        let fa = char::from_u32('۰' as u32 + (*d as u32 - '0' as u32)).unwrap_or(*d);
        out.push(fa);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toman_roundtrip() {
        assert_eq!(Rial::from_toman(125_000).to_toman(), 125_000);
        assert_eq!(Rial::from_toman(125_000).0, 1_250_000);
    }

    #[test]
    fn format_fa() {
        assert_eq!(
            Rial::from_toman(1_250_000).format_toman_fa(),
            "۱٬۲۵۰٬۰۰۰ تومان"
        );
        assert_eq!(Rial::from_toman(0).format_toman_fa(), "۰ تومان");
    }
}

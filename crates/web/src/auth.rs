//! احراز هویت: تولید/تأیید OTP، هش رمز (argon2id)، و نشست مبتنی بر کوکی امضاشده.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;

/// تولید کد OTP پنج‌رقمی.
pub fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    format!("{:05}", rng.gen_range(0..100_000))
}

/// تولید کد اتصال تلگرام: ۸ کاراکتر از الفبای بدون‌ابهام (بدون 0/O/1/I).
pub fn generate_link_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// تولید توکن نشست: ۶۴ کاراکتر هگز تصادفی (۳۲ بایت).
/// طولِ ۶۴ آن را از کوکی‌های قدیمیِ حاوی UUID (با خط‌تیره، طول ۳۶) متمایز می‌کند.
pub fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// هش یک رشته (OTP یا رمز) با argon2id.
pub fn hash_secret(secret: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(secret.as_bytes(), &salt)?.to_string())
}

/// تأیید یک رشته در برابر هش ذخیره‌شده.
pub fn verify_secret(secret: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// نام کوکی‌ها.
pub const SESSION_COOKIE: &str = "zagros_session";
pub const CART_COOKIE: &str = "zagros_cart";

/// نرمال‌سازی شماره موبایل ایرانی به فرم `09xxxxxxxxx`.
pub fn normalize_mobile(input: &str) -> Option<String> {
    // فقط ارقام را نگه می‌داریم (با تبدیل ارقام فارسی).
    let digits: String = input
        .chars()
        .filter_map(|c| {
            if c.is_ascii_digit() {
                Some(c)
            } else if ('۰'..='۹').contains(&c) {
                char::from_u32('0' as u32 + (c as u32 - '۰' as u32))
            } else {
                None
            }
        })
        .collect();

    let normalized = if let Some(rest) = digits.strip_prefix("98") {
        format!("0{rest}")
    } else if digits.starts_with("0") {
        digits
    } else if digits.len() == 10 && digits.starts_with('9') {
        format!("0{digits}")
    } else {
        digits
    };

    if normalized.len() == 11 && normalized.starts_with("09") {
        Some(normalized)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_is_five_digits() {
        let otp = generate_otp();
        assert_eq!(otp.len(), 5);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn hash_and_verify() {
        let h = hash_secret("12345").unwrap();
        assert!(verify_secret("12345", &h));
        assert!(!verify_secret("99999", &h));
    }

    #[test]
    fn mobile_normalization() {
        assert_eq!(
            normalize_mobile("09123456789").as_deref(),
            Some("09123456789")
        );
        assert_eq!(
            normalize_mobile("+989123456789").as_deref(),
            Some("09123456789")
        );
        assert_eq!(
            normalize_mobile("9123456789").as_deref(),
            Some("09123456789")
        );
        assert_eq!(
            normalize_mobile("۰۹۱۲۳۴۵۶۷۸۹").as_deref(),
            Some("09123456789")
        );
        assert_eq!(normalize_mobile("123"), None);
    }
}

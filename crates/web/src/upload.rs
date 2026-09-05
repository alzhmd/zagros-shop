//! ذخیره‌ی فایل‌های آپلودی (تصویر محصول) روی دیسک محلی.
//! خودمیزبان، بدون سرویس خارجی. مسیر سرو: `/assets/uploads/<file>`.

use axum::extract::multipart::Field;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const UPLOAD_DIR: &str = "assets/uploads";
const MAX_BYTES: usize = 6 * 1024 * 1024; // ۶ مگابایت

/// پسوند مجاز را از نوع محتوا یا نام فایل تشخیص می‌دهد.
fn ext_for(content_type: Option<&str>, filename: Option<&str>) -> Option<&'static str> {
    let by_ct = match content_type {
        Some("image/jpeg") => Some("jpg"),
        Some("image/png") => Some("png"),
        Some("image/webp") => Some("webp"),
        Some("image/gif") => Some("gif"),
        Some("image/avif") => Some("avif"),
        _ => None,
    };
    if by_ct.is_some() {
        return by_ct;
    }
    let name = filename?.to_lowercase();
    if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        Some("jpg")
    } else if name.ends_with(".png") {
        Some("png")
    } else if name.ends_with(".webp") {
        Some("webp")
    } else if name.ends_with(".gif") {
        Some("gif")
    } else if name.ends_with(".avif") {
        Some("avif")
    } else {
        None
    }
}

/// یک فیلد multipart را به‌عنوان تصویر ذخیره می‌کند و مسیر عمومی را برمی‌گرداند.
/// در صورت نبود تصویر معتبر یا حجم زیاد، `Ok(None)`.
pub async fn save_image_field(field: Field<'_>) -> Result<Option<String>, String> {
    let content_type = field.content_type().map(|s| s.to_string());
    let filename = field.file_name().map(|s| s.to_string());
    let Some(ext) = ext_for(content_type.as_deref(), filename.as_deref()) else {
        return Ok(None);
    };

    let data = field
        .bytes()
        .await
        .map_err(|e| format!("خواندن فایل شکست خورد: {e}"))?;
    if data.is_empty() {
        return Ok(None);
    }
    if data.len() > MAX_BYTES {
        return Err("حجم تصویر بیش از ۶ مگابایت است.".into());
    }

    tokio::fs::create_dir_all(UPLOAD_DIR)
        .await
        .map_err(|e| format!("ساخت پوشه‌ی آپلود شکست خورد: {e}"))?;

    let name = format!("{}.{}", Uuid::new_v4(), ext);
    let path = Path::new(UPLOAD_DIR).join(&name);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| format!("ساخت فایل شکست خورد: {e}"))?;
    file.write_all(&data)
        .await
        .map_err(|e| format!("نوشتن فایل شکست خورد: {e}"))?;

    Ok(Some(format!("/assets/uploads/{name}")))
}

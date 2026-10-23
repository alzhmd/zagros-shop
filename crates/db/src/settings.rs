//! Repository تنظیمات سایت — جدول key/value با مقدار jsonb.
//! مقادیر قابل‌ویرایش در زمان اجرا (نام سایت، هزینه‌ی ارسال، نرخ امتیاز، …).
//! رازها (توکن ربات، کلید مرچنت) اینجا نیستند — از ENV/config می‌آیند.

use crate::Db;
use std::collections::HashMap;

/// خواندن همه‌ی تنظیمات به‌صورت map از key به مقدار رشته‌ای.
/// مقادیر jsonb رشته‌ای به String و سایر انواع به نمایش JSON تبدیل می‌شوند.
pub async fn get_all(db: &Db) -> Result<HashMap<String, String>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, serde_json::Value)>("SELECT key, value FROM settings")
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(k, v)| (k, json_to_string(&v)))
        .collect())
}

/// مقدار یک کلید (در صورت وجود).
pub async fn get(db: &Db, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row =
        sqlx::query_scalar::<_, serde_json::Value>("SELECT value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|v| json_to_string(&v)))
}

/// تنظیم/به‌روزرسانی یک کلید (مقدار به‌صورت رشته‌ی JSON ذخیره می‌شود).
pub async fn set(db: &Db, key: &str, value: &str) -> Result<(), sqlx::Error> {
    let json = serde_json::Value::String(value.to_string());
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, now())
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(json)
    .execute(db)
    .await?;
    Ok(())
}

/// تبدیل مقدار jsonb به رشته: رشته‌ها بدون گیومه، بقیه به‌صورت JSON.
fn json_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

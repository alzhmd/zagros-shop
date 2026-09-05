use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Category {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Brand {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub logo: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Product {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub brand_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub short_desc: Option<String>,
    pub description: Option<String>,
    pub base_price: i64,
    pub is_active: bool,
    pub is_featured: bool,
    pub view_count: i64,
    pub video_url: Option<String>,
    pub deal_ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ProductVariant {
    pub id: Uuid,
    pub product_id: Uuid,
    pub sku: Option<String>,
    pub title: String,
    pub price: i64,
    /// قیمت قبل از تخفیف (ریال). اگر > price باشد، تخفیف فعال است.
    pub compare_at_price: Option<i64>,
    pub stock: i32,
    pub is_active: bool,
    pub sort_order: i32,
}

impl ProductVariant {
    /// قیمت قالب‌بندی‌شده‌ی فارسی برای نمایش.
    pub fn price_fa(&self) -> String {
        domain::Rial(self.price).format_toman_fa()
    }
    /// آیا این تنوع تخفیف دارد؟
    pub fn has_discount(&self) -> bool {
        self.compare_at_price.map_or(false, |c| c > self.price)
    }
    /// قیمت قبل از تخفیف قالب‌بندی‌شده.
    pub fn compare_at_fa(&self) -> String {
        domain::Rial(self.compare_at_price.unwrap_or(0)).format_toman_fa()
    }
    /// درصد تخفیف (گردشده).
    pub fn discount_pct(&self) -> i64 {
        match self.compare_at_price {
            Some(c) if c > self.price && c > 0 => ((c - self.price) * 100 + c / 2) / c,
            _ => 0,
        }
    }
    pub fn discount_pct_fa(&self) -> String {
        domain::fa_digits(&self.discount_pct().to_string())
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ProductImage {
    pub id: Uuid,
    pub product_id: Uuid,
    pub path: String,
    pub alt: Option<String>,
    pub sort_order: i32,
    pub is_primary: bool,
}

/// محصول به‌همراه اطلاعات نمایشی برای کارت کاتالوگ (با join).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ProductCard {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub short_desc: Option<String>,
    pub base_price: i64,
    /// قیمت فروش واقعی = ارزان‌ترین تنوع فعال (در صورت نبود، base_price).
    pub min_price: Option<i64>,
    /// قیمت قبل از تخفیفِ همان تنوع ارزان (برای بَج تخفیف روی کارت).
    pub compare_at: Option<i64>,
    pub brand_name: Option<String>,
    pub category_slug: Option<String>,
    pub primary_image: Option<String>,
    pub in_stock: bool,
    pub rating_avg: f64,
    pub rating_count: i64,
}

impl ProductCard {
    /// قیمت فروش (ریال) — ارزان‌ترین تنوع یا base_price.
    pub fn sell_price(&self) -> i64 {
        self.min_price.unwrap_or(self.base_price)
    }
    /// قیمت قالب‌بندی‌شده‌ی فارسی برای نمایش در قالب: «۱۸٬۵۰۰٬۰۰۰ تومان».
    pub fn price_fa(&self) -> String {
        domain::Rial(self.sell_price()).format_toman_fa()
    }
    /// آیا تخفیف دارد؟
    pub fn has_discount(&self) -> bool {
        self.compare_at.map_or(false, |c| c > self.sell_price())
    }
    /// قیمت قبل از تخفیف قالب‌بندی‌شده.
    pub fn compare_at_fa(&self) -> String {
        domain::Rial(self.compare_at.unwrap_or(0)).format_toman_fa()
    }
    /// درصد تخفیف (گردشده).
    pub fn discount_pct(&self) -> i64 {
        match self.compare_at {
            Some(c) if c > self.sell_price() && c > 0 => ((c - self.sell_price()) * 100 + c / 2) / c,
            _ => 0,
        }
    }
    pub fn discount_pct_fa(&self) -> String {
        domain::fa_digits(&self.discount_pct().to_string())
    }
    /// آیا نقد تأییدشده‌ای دارد؟
    pub fn has_rating(&self) -> bool {
        self.rating_count > 0
    }
    /// ستاره‌ها به‌صورت رشته (★ پر + ☆ خالی) بر پایه‌ی میانگین گردشده.
    pub fn rating_stars(&self) -> String {
        let full = self.rating_avg.round().clamp(0.0, 5.0) as usize;
        "★".repeat(full) + &"☆".repeat(5 - full)
    }
    pub fn rating_count_fa(&self) -> String {
        domain::fa_digits(&self.rating_count.to_string())
    }
}

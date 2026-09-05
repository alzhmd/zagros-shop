-- فاز ارتقا: ایندکس‌های کمکی برای مسیرهای داغ (پیشنهادها، پرفروش‌ها، جدیدترین‌ها).

-- پربازدیدترین‌ها (صفحه‌ی پیشنهادها) و مرتب‌سازی محبوبیت.
CREATE INDEX IF NOT EXISTS idx_products_active_view
    ON products(view_count DESC) WHERE is_active;

-- جدیدترین‌ها / مرتب‌سازی پیش‌فرض کاتالوگ.
CREATE INDEX IF NOT EXISTS idx_products_active_created
    ON products(created_at DESC) WHERE is_active;

-- جست‌وجوی ادمین روی نامک/قیمت.
CREATE INDEX IF NOT EXISTS idx_products_base_price ON products(base_price);

-- علاقه‌مندی‌ها بر اساس محصول (برای شمارش/پاکسازی).
CREATE INDEX IF NOT EXISTS idx_wishlist_product ON wishlists(product_id);

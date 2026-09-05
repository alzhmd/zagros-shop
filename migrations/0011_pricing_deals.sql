-- فاز ارتقا U1: مدل تخفیف — قیمت قبل (compare_at) در سطح تنوع + تایمر پیشنهاد لحظه‌ای محصول.

-- قیمت قبل از تخفیف برای هر تنوع (ریال). NULL = بدون تخفیف.
-- اگر > price باشد، تخفیف فعال است و درصد محاسبه می‌شود.
ALTER TABLE product_variants ADD COLUMN compare_at_price bigint;

-- پایان مهلت پیشنهاد لحظه‌ای («شگفت‌انگیز»). NULL = بدون تایمر.
ALTER TABLE products ADD COLUMN deal_ends_at timestamptz;

-- ایندکس محصولات دارای تایمر فعال (برای ریل «پیشنهاد لحظه‌ای» در خانه).
CREATE INDEX idx_products_deal_ends ON products(deal_ends_at)
    WHERE deal_ends_at IS NOT NULL;

-- نمونه: یک تخفیف روی جاروبرقی پارس خزر + تایمر، تا مسیر تخفیف از روز اول دیده شود.
UPDATE product_variants SET compare_at_price = 23900000
    WHERE sku = 'PK-VC2000';
UPDATE products SET deal_ends_at = now() + interval '2 days'
    WHERE slug = 'parskhazar-vc-2000';

UPDATE product_variants SET compare_at_price = 199000000
    WHERE sku = 'SNW-S8-2261-SLV';

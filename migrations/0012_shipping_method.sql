-- فاز ارتقا U1: روش ارسال انتخابی روی سفارش + تنظیمات هزینه‌ی روش‌های ارسال.

-- روش ارسال انتخاب‌شده (snapshot نام نمایشی + کلید).
ALTER TABLE orders ADD COLUMN shipping_method text;

-- تنظیمات روش‌های ارسال (هزینه به تومان، زمان تقریبی). از پنل ادمین قابل‌ویرایش.
-- کلیدها: shipping.<key>.enabled | .label | .toman | .eta
INSERT INTO settings (key, value) VALUES
    ('shipping.normal.enabled',  '"true"'),
    ('shipping.normal.label',    '"ارسال عادی"'),
    ('shipping.normal.toman',    '"0"'),
    ('shipping.normal.eta',      '"۳ تا ۵ روز کاری"'),
    ('shipping.express.enabled', '"true"'),
    ('shipping.express.label',   '"ارسال پیشتاز"'),
    ('shipping.express.toman',   '"50000"'),
    ('shipping.express.eta',     '"۱ تا ۲ روز کاری"'),
    ('shipping.instant.enabled', '"false"'),
    ('shipping.instant.label',   '"ارسال فوری (همان روز)"'),
    ('shipping.instant.toman',   '"120000"'),
    ('shipping.instant.eta',     '"همان روز — فقط زنجان"')
ON CONFLICT (key) DO NOTHING;

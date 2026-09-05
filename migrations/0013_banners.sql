-- فاز ارتقا U2: بنر/کمپین مدیریت‌پذیر — اسلایدر صفحه‌ی اصلی و بنرهای جایگاهی.

CREATE TABLE banners (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    placement   text NOT NULL DEFAULT 'hero',   -- hero | strip | grid (جایگاه نمایش)
    title       text,
    subtitle    text,
    image_path  text,                            -- تصویر بنر (اختیاری؛ در نبود، گرادیان)
    link        text,                            -- مقصد کلیک
    cta_label   text,                            -- متن دکمه
    bg_color    text,                            -- رنگ پس‌زمینه‌ی اختیاری (#hex)
    sort_order  int NOT NULL DEFAULT 0,
    is_active   boolean NOT NULL DEFAULT true,
    starts_at   timestamptz,
    ends_at     timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_banners_placement ON banners(placement, sort_order) WHERE is_active;

-- چند اسلاید نمونه برای صفحه‌ی اصلی (بدون تصویر — گرادیان CSS).
INSERT INTO banners (placement, title, subtitle, link, cta_label, sort_order) VALUES
    ('hero', 'سرمای مطمئن، خدمات تخصصی', 'یخچال، فریزر و آبسردکن از بهترین برندها با ضمانت اصالت کالا', '/products', 'مشاهده‌ی محصولات', 1),
    ('hero', 'پیشنهادهای شگفت‌انگیز زاگرس', 'تخفیف‌های روزانه روی محصولات منتخب — تا فرصت باقی‌ست', '/offers', 'دیدن پیشنهادها', 2),
    ('hero', 'دستگاهتان خراب شده؟', 'درخواست تعمیر آنلاین ثبت کنید؛ اعزام کارشناس یا پذیرش در انبار', '/repair', 'ثبت درخواست تعمیر', 3),
    ('strip', 'کد تخفیف خوش‌آمدگویی: WELCOME10', 'اولین خرید خود را با ۱۰٪ تخفیف ثبت کنید', '/products', 'همین حالا خرید کنید', 1);

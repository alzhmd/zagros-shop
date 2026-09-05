-- فاز ۵: باشگاه مشتریان — کد تخفیف، دفتر امتیاز، و اتصال حساب به تلگرام.
-- یادآوری: users.points (bigint) و users.telegram_id (bigint UNIQUE) از migration 0004 موجودند.

-- ===================== کد تخفیف =====================
CREATE TABLE coupons (
    id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    code          text NOT NULL UNIQUE,           -- کد یکتا (حروف بزرگ نرمال‌شده)
    description   text,
    kind          text NOT NULL DEFAULT 'percent', -- percent | fixed
    value         bigint NOT NULL,                 -- درصد (۱..۱۰۰) یا مبلغ ثابت ریالی
    min_order     bigint NOT NULL DEFAULT 0,        -- حداقل مبلغ سفارش (ریال) برای اعمال
    max_discount  bigint,                           -- سقف تخفیف ریالی (برای نوع درصدی)؛ NULL = بی‌سقف
    usage_limit   int,                              -- حداکثر دفعات استفاده‌ی کل؛ NULL = نامحدود
    used_count    int NOT NULL DEFAULT 0,
    per_user_limit int NOT NULL DEFAULT 1,          -- حداکثر دفعه برای هر کاربر
    starts_at     timestamptz,
    expires_at    timestamptz,
    is_active     boolean NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    CHECK (value > 0),
    CHECK (kind IN ('percent', 'fixed'))
);
CREATE INDEX idx_coupons_active ON coupons(is_active);

-- استفاده‌های هر کاربر از کوپن (برای سقف per_user و گزارش).
CREATE TABLE coupon_redemptions (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    coupon_id   uuid NOT NULL REFERENCES coupons(id) ON DELETE CASCADE,
    user_id     uuid REFERENCES users(id) ON DELETE SET NULL,
    order_id    uuid REFERENCES orders(id) ON DELETE SET NULL,
    amount      bigint NOT NULL,                    -- مبلغ تخفیف اعمال‌شده (ریال)
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_coupon_redemptions_coupon ON coupon_redemptions(coupon_id);
CREATE INDEX idx_coupon_redemptions_user ON coupon_redemptions(user_id);

-- ===================== دفتر امتیاز باشگاه =====================
-- دفتر تراکنش‌های امتیاز؛ users.points همیشه برابر مجموع این دفتر نگه داشته می‌شود.
CREATE TABLE loyalty_transactions (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta       bigint NOT NULL,                    -- مثبت = کسب، منفی = خرج
    reason      text NOT NULL,                      -- purchase | telegram | signup | manual | redeem
    ref_order   uuid REFERENCES orders(id) ON DELETE SET NULL,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_loyalty_user ON loyalty_transactions(user_id, created_at DESC);
-- هر سفارش فقط یک‌بار امتیاز خرید بگیرد (idempotent در برابر callback تکراری پرداخت).
CREATE UNIQUE INDEX idx_loyalty_purchase_once
    ON loyalty_transactions(ref_order) WHERE reason = 'purchase';

-- ===================== کد اتصال حساب به تلگرام =====================
-- کاربر در سایت کد می‌گیرد و در ربات /start <code> می‌زند تا telegram_id به حسابش وصل شود.
CREATE TABLE telegram_link_codes (
    code        text PRIMARY KEY,                   -- کد کوتاه یکبارمصرف
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at  timestamptz NOT NULL,
    consumed    boolean NOT NULL DEFAULT false,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_tg_link_user ON telegram_link_codes(user_id);

-- چند کوپن نمونه برای تست/راه‌اندازی.
INSERT INTO coupons (code, description, kind, value, min_order, max_discount, per_user_limit) VALUES
    ('WELCOME10', 'تخفیف ۱۰٪ خوش‌آمدگویی', 'percent', 10, 0, 5000000, 1),
    ('ZAGROS500', 'تخفیف ثابت ۵۰٬۰۰۰ تومان', 'fixed', 5000000, 30000000, NULL, 3);

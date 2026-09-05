-- فاز ارتقا U5: فیلدهای پروفایل، کارت هدیه، ذخیره برای بعد، تنظیمات سطوح باشگاه.

-- پروفایل
ALTER TABLE users ADD COLUMN birthdate date;
ALTER TABLE users ADD COLUMN gender text;   -- male | female | '' (نامشخص)

-- ذخیره برای بعد در سبد
ALTER TABLE cart_items ADD COLUMN saved_for_later boolean NOT NULL DEFAULT false;

-- کارت هدیه (کد یکتا؛ پس از وارد کردن، موجودی به کیف پول واریز می‌شود).
CREATE TABLE gift_cards (
    id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    code         text NOT NULL UNIQUE,
    initial_rial bigint NOT NULL,
    balance_rial bigint NOT NULL,
    issued_to    uuid REFERENCES users(id) ON DELETE SET NULL,   -- گیرنده‌ی اولیه (در صورت صدور خودکار)
    redeemed_by  uuid REFERENCES users(id) ON DELETE SET NULL,   -- مصرف‌کننده
    redeemed_at  timestamptz,
    expires_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_giftcards_issued ON gift_cards(issued_to);

-- تنظیمات سطوح باشگاه (آستانه‌ی امتیاز) + کارت هدیه‌ی خوش‌آمد.
INSERT INTO settings (key, value) VALUES
    ('club.silver_threshold', '"200"'),
    ('club.gold_threshold',   '"1000"'),
    ('giftcard.signup_enabled', '"false"'),
    ('giftcard.signup_toman',   '"50000"'),
    ('giftcard.signup_days',    '"90"')
ON CONFLICT (key) DO NOTHING;

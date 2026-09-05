-- فاز ارتقا U4: اعلان‌ها + هشدار موجودشدن/کاهش قیمت.

CREATE TABLE notifications (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind        text NOT NULL,                      -- order | review | coupon | club | stock | price | system
    title       text NOT NULL,
    body        text,
    url         text,                               -- مقصد کلیک (اختیاری)
    read_at     timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_notif_user ON notifications(user_id, created_at DESC);
CREATE INDEX idx_notif_unread ON notifications(user_id) WHERE read_at IS NULL;

-- هشدار موجودشدن: کاربر برای یک تنوعِ ناموجود ثبت‌نام می‌کند.
CREATE TABLE stock_alerts (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    variant_id  uuid NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    notified_at timestamptz,
    UNIQUE (user_id, variant_id)
);
CREATE INDEX idx_stock_alerts_variant ON stock_alerts(variant_id) WHERE notified_at IS NULL;

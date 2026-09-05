-- فاز ارتقا U3: کیف پول — موجودی روی users + دفتر تراکنش (آینه‌ی الگوی باشگاه).

ALTER TABLE users ADD COLUMN wallet_balance bigint NOT NULL DEFAULT 0;

CREATE TABLE wallet_transactions (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta       bigint NOT NULL,                    -- مثبت = واریز، منفی = برداشت (ریال)
    kind        text NOT NULL,                      -- topup | spend | cancel_refund | return_refund | giftcard | admin
    ref_order   uuid REFERENCES orders(id) ON DELETE SET NULL,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_wallet_user ON wallet_transactions(user_id, created_at DESC);
-- هر سفارش فقط یک‌بار بازگشتِ لغو بگیرد (idempotent در برابر فراخوانی تکراری).
CREATE UNIQUE INDEX idx_wallet_cancel_once
    ON wallet_transactions(ref_order) WHERE kind = 'cancel_refund';

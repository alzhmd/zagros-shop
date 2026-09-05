-- فاز ارتقا: سیستم مرجوعی/بازگشت کالا (Returns / Refunds) — مثل دیجی‌کالا/آمازون.

CREATE TABLE returns (
    id             uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    return_number  bigserial UNIQUE,                 -- شماره‌ی خوانا برای مشتری
    order_id       uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    user_id        uuid REFERENCES users(id) ON DELETE SET NULL,
    reason         text NOT NULL,                    -- defective | wrong | changed_mind | other
    description    text,
    status         text NOT NULL DEFAULT 'requested',
       -- requested | approved | rejected | received | refunded | cancelled
    refund_amount  bigint,                           -- مبلغ بازگشتی (ریال)؛ NULL تا تعیین‌نشده
    admin_note     text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_returns_user ON returns(user_id, created_at DESC);
CREATE INDEX idx_returns_order ON returns(order_id);
CREATE INDEX idx_returns_status ON returns(status);

-- اقلام مرجوعی (snapshot از قلم سفارش).
CREATE TABLE return_items (
    id             uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    return_id      uuid NOT NULL REFERENCES returns(id) ON DELETE CASCADE,
    order_item_id  uuid REFERENCES order_items(id) ON DELETE SET NULL,
    product_name   text NOT NULL,
    variant_title  text,
    qty            int NOT NULL,
    unit_price     bigint NOT NULL
);
CREATE INDEX idx_return_items_return ON return_items(return_id);

-- تاریخچه‌ی وضعیت مرجوعی.
CREATE TABLE return_status_log (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    return_id   uuid NOT NULL REFERENCES returns(id) ON DELETE CASCADE,
    status      text NOT NULL,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_return_log_return ON return_status_log(return_id, created_at);

CREATE TRIGGER trg_returns_touch BEFORE UPDATE ON returns
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

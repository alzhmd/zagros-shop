-- فاز ۲: کاربران، نشست، سبد، سفارش، پرداخت.

-- کاربران (ورود با موبایل؛ ایمیل و رمز اختیاری).
CREATE TABLE users (
    id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    mobile       text NOT NULL UNIQUE,
    email        text UNIQUE,
    full_name    text,
    password_hash text,                       -- argon2id؛ NULL اگر فقط با OTP وارد می‌شود
    role         text NOT NULL DEFAULT 'customer',  -- customer | admin | operator | technician
    points       bigint NOT NULL DEFAULT 0,        -- امتیاز باشگاه مشتریان
    telegram_id  bigint UNIQUE,                     -- اتصال به ربات (فاز ۵)
    is_active    boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_users_role ON users(role);

-- کدهای یکبارمصرف ورود (OTP).
CREATE TABLE otp_codes (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    mobile      text NOT NULL,
    code_hash   text NOT NULL,                  -- کد به‌صورت هش‌شده ذخیره می‌شود
    expires_at  timestamptz NOT NULL,
    attempts    int NOT NULL DEFAULT 0,
    consumed    boolean NOT NULL DEFAULT false,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_otp_mobile ON otp_codes(mobile, created_at DESC);

-- آدرس‌های کاربر (پرداخت در محل فقط استان زنجان).
CREATE TABLE addresses (
    id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver     text NOT NULL,
    phone        text NOT NULL,
    province     text NOT NULL,
    city         text NOT NULL,
    postal_code  text,
    line         text NOT NULL,
    is_default   boolean NOT NULL DEFAULT false,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_addresses_user ON addresses(user_id);

-- سبد خرید (هم برای کاربر لاگین‌شده، هم مهمان با session token).
CREATE TABLE carts (
    id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      uuid REFERENCES users(id) ON DELETE CASCADE,
    session_key  text,                          -- برای مهمان
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_carts_user ON carts(user_id);
CREATE INDEX idx_carts_session ON carts(session_key);

CREATE TABLE cart_items (
    id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    cart_id      uuid NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    variant_id   uuid NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    qty          int NOT NULL CHECK (qty > 0),
    UNIQUE (cart_id, variant_id)
);
CREATE INDEX idx_cart_items_cart ON cart_items(cart_id);

-- سفارش‌ها.
CREATE TABLE orders (
    id             uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_number   bigserial UNIQUE,            -- شماره‌ی خوانا برای مشتری
    user_id        uuid REFERENCES users(id) ON DELETE SET NULL,
    status         text NOT NULL DEFAULT 'pending',
       -- pending | awaiting_payment | paid | processing | shipped | delivered | cancelled | cod_pending
    payment_method text NOT NULL DEFAULT 'online',  -- online | cod (پرداخت در محل، فقط زنجان)
    subtotal       bigint NOT NULL DEFAULT 0,       -- جمع اقلام (ریال)
    discount       bigint NOT NULL DEFAULT 0,
    shipping_cost  bigint NOT NULL DEFAULT 0,
    total          bigint NOT NULL DEFAULT 0,
    -- snapshot آدرس (تا تغییر بعدی آدرس روی سفارش اثر نگذارد)
    ship_receiver  text,
    ship_phone     text,
    ship_province  text,
    ship_city      text,
    ship_address   text,
    coupon_code    text,
    note           text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_orders_user ON orders(user_id, created_at DESC);
CREATE INDEX idx_orders_status ON orders(status);

-- اقلام سفارش (snapshot قیمت و نام در لحظه‌ی خرید).
CREATE TABLE order_items (
    id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id      uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    variant_id    uuid REFERENCES product_variants(id) ON DELETE SET NULL,
    product_name  text NOT NULL,
    variant_title text,
    unit_price    bigint NOT NULL,
    qty           int NOT NULL,
    line_total    bigint NOT NULL
);
CREATE INDEX idx_order_items_order ON order_items(order_id);

-- پرداخت‌ها (هسته‌ی انتزاع درگاه؛ idempotent با gateway_token).
CREATE TABLE payments (
    id             uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id       uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    provider       text NOT NULL,               -- zarinpal | zibal
    gateway_token  text,                         -- authority (زرین‌پال) یا trackId (زیبال)
    amount         bigint NOT NULL,              -- ریال
    state          text NOT NULL DEFAULT 'created',  -- created | redirected | paid | failed | verified
    ref_id         text,                         -- شماره‌ی پیگیری بانک پس از تأیید
    card_pan       text,
    raw_response   jsonb,
    created_at     timestamptz NOT NULL DEFAULT now(),
    verified_at    timestamptz
);
CREATE INDEX idx_payments_order ON payments(order_id);
CREATE INDEX idx_payments_token ON payments(provider, gateway_token);

-- تریگر updated_at برای سفارش و سبد.
CREATE TRIGGER trg_orders_touch BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER trg_carts_touch BEFORE UPDATE ON carts
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

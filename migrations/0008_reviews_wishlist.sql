-- فاز ارتقا: نقد و امتیاز، علاقه‌مندی‌ها، ویدئوی محصول، و یادداشت‌های پروفایل.

-- ===================== نقد و امتیاز محصول =====================
CREATE TABLE product_reviews (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id  uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    user_id     uuid REFERENCES users(id) ON DELETE SET NULL,
    author_name text NOT NULL,
    rating      int NOT NULL CHECK (rating BETWEEN 1 AND 5),
    title       text,
    body        text NOT NULL,
    is_verified boolean NOT NULL DEFAULT false,   -- خرید تأییدشده
    status      text NOT NULL DEFAULT 'pending',  -- pending | approved | rejected
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_reviews_product ON product_reviews(product_id, status);
CREATE INDEX idx_reviews_status ON product_reviews(status, created_at DESC);
-- هر کاربر فقط یک نقد فعال برای هر محصول.
CREATE UNIQUE INDEX idx_reviews_user_product
    ON product_reviews(product_id, user_id) WHERE user_id IS NOT NULL;

-- ===================== علاقه‌مندی‌ها (Wishlist) =====================
CREATE TABLE wishlists (
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id  uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, product_id)
);
CREATE INDEX idx_wishlist_user ON wishlists(user_id, created_at DESC);

-- ===================== ویدئوی محصول =====================
ALTER TABLE products ADD COLUMN video_url text;

-- ===================== تنظیمات پیش‌فرض افزوده =====================
INSERT INTO settings (key, value) VALUES
    ('site.contact_phone',   '""'),
    ('site.contact_address', '"استان زنجان — ارسال به سراسر کشور"'),
    ('shipping.flat_toman',  '"0"'),
    ('cod.province',         '"زنجان"'),
    ('loyalty.rial_per_point','"100000"'),
    ('payments.enabled',     '"false"')
ON CONFLICT (key) DO NOTHING;

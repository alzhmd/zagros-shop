-- فاز ارتقا U7: امتیاز «مفید بودن» دیدگاه + مرکز دیدگاه‌های کاربر.

-- شمارنده‌ی «مفید بود» روی هر نقد.
ALTER TABLE product_reviews ADD COLUMN helpful_count int NOT NULL DEFAULT 0;

-- رأی «مفید بود» — هر کاربر یک‌بار برای هر نقد.
CREATE TABLE review_helpful (
    review_id  uuid NOT NULL REFERENCES product_reviews(id) ON DELETE CASCADE,
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (review_id, user_id)
);

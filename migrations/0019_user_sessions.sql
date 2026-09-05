-- فاز ارتقا U8: نشست‌های فعال (مدیریت دستگاه‌های واردشده + امنیت).
-- نشست به‌جای ذخیره‌ی مستقیم user_id در کوکی، با یک توکن تصادفی شناخته می‌شود.
-- (کوکی‌های قدیمیِ حاوی UUID همچنان معتبرند — سازگاری رو به عقب در لایه‌ی extract.)

CREATE TABLE user_sessions (
    token        text PRIMARY KEY,            -- ۶۴ هگز تصادفی (اعتبارنامه — هرگز در HTML نمایش داده نمی‌شود)
    id           uuid NOT NULL DEFAULT uuid_generate_v4() UNIQUE,  -- شناسه‌ی عمومی برای لغو
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_agent   text,                        -- رشته‌ی User-Agent مرورگر
    ip           text,                        -- آی‌پی هنگام ورود (از X-Forwarded-For)
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_sessions_user ON user_sessions(user_id, last_seen_at DESC);

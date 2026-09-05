-- مهاجرت اولیه‌ی فاز ۰: اکستنشن‌ها، تابع نرمال‌سازی فارسی، و جدول تنظیمات.

CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- نرمال‌سازی متن فارسی: یکسان‌سازی ی/ک عربی↔فارسی، ارقام، و حذف ZWNJ.
-- IMMUTABLE تا در ایندکس و ستون ژنریت قابل استفاده باشد.
CREATE OR REPLACE FUNCTION fa_normalize(t text) RETURNS text AS $$
  SELECT regexp_replace(
           translate(
             lower(coalesce($1, '')),
             'يىكۀٱأإآ' || '۰۱۲۳۴۵۶۷۸۹' || '٠١٢٣٤٥٦٧٨٩',
             'ییککاااا'  || '0123456789' || '0123456789'
           ),
           E'‌', ' ', 'g'
         );
$$ LANGUAGE sql IMMUTABLE;

-- تنظیمات کلید-مقدار (درگاه فعال، پیامک فعال، و…).
CREATE TABLE settings (
    key         text PRIMARY KEY,
    value       jsonb NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now()
);

INSERT INTO settings (key, value) VALUES
    ('site.name',        '"زاگرس — جهان کولاک"'),
    ('payments.active',  '"zibal"'),
    ('sms.active',       '"none"'),
    ('search.engine',    '"postgres"')
ON CONFLICT (key) DO NOTHING;

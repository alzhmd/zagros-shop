-- فاز ۱: مدل داده‌ی کاتالوگ — دسته‌بندی، برند، محصول، تنوع، تصویر، ویژگی.

-- دسته‌بندی درختی (والد خودارجاع).
CREATE TABLE categories (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id   uuid REFERENCES categories(id) ON DELETE SET NULL,
    name        text NOT NULL,
    slug        text NOT NULL UNIQUE,
    description text,
    icon        text,
    sort_order  int NOT NULL DEFAULT 0,
    is_active   boolean NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_categories_parent ON categories(parent_id);
CREATE INDEX idx_categories_active ON categories(is_active) WHERE is_active;

-- برند.
CREATE TABLE brands (
    id         uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    name       text NOT NULL,
    slug       text NOT NULL UNIQUE,
    logo       text,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- محصول (موجودیت اصلی کاتالوگ).
CREATE TABLE products (
    id              uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    category_id     uuid REFERENCES categories(id) ON DELETE SET NULL,
    brand_id        uuid REFERENCES brands(id) ON DELETE SET NULL,
    name            text NOT NULL,
    slug            text NOT NULL UNIQUE,
    short_desc      text,
    description     text,
    -- قیمت پایه به ریال (تنوع‌ها می‌توانند override کنند). پول همیشه صحیح، نه اعشاری.
    base_price      bigint NOT NULL DEFAULT 0,
    is_active       boolean NOT NULL DEFAULT true,
    is_featured     boolean NOT NULL DEFAULT false,
    view_count      bigint NOT NULL DEFAULT 0,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    -- ستون نرمال‌شده برای جستجوی فارسی (ی/ک، ارقام، ZWNJ یکسان‌سازی شده).
    name_norm       text GENERATED ALWAYS AS (fa_normalize(name)) STORED
);
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_brand ON products(brand_id);
CREATE INDEX idx_products_active ON products(is_active) WHERE is_active;
CREATE INDEX idx_products_featured ON products(is_featured) WHERE is_featured;
-- ایندکس trigram برای جستجوی فازی فارسی روی نام نرمال‌شده.
CREATE INDEX idx_products_name_norm_trgm ON products USING GIN (name_norm gin_trgm_ops);

-- تنوع محصول (رنگ/مدل/سایز با قیمت و موجودی مستقل).
CREATE TABLE product_variants (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id  uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku         text UNIQUE,
    title       text NOT NULL,
    price       bigint NOT NULL,
    stock       int NOT NULL DEFAULT 0,
    is_active   boolean NOT NULL DEFAULT true,
    sort_order  int NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_variants_product ON product_variants(product_id);

-- تصاویر محصول (مسیر WebP/AVIF تولیدشده، ترتیب نمایش).
CREATE TABLE product_images (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id  uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    path        text NOT NULL,
    alt         text,
    sort_order  int NOT NULL DEFAULT 0,
    is_primary  boolean NOT NULL DEFAULT false
);
CREATE INDEX idx_images_product ON product_images(product_id);

-- تعریف ویژگی‌های قابل‌فیلتر (مثل «حجم»، «نوع»، «رنگ»).
CREATE TABLE attributes (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        text NOT NULL,
    slug        text NOT NULL UNIQUE,
    -- آیا در نوار فیلتر نمایش داده شود؟
    is_filterable boolean NOT NULL DEFAULT true,
    sort_order  int NOT NULL DEFAULT 0
);

-- مقدار ویژگی برای هر محصول (مثل حجم=۲۲ فوت).
CREATE TABLE product_attributes (
    product_id   uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    attribute_id uuid NOT NULL REFERENCES attributes(id) ON DELETE CASCADE,
    value        text NOT NULL,
    value_norm   text GENERATED ALWAYS AS (fa_normalize(value)) STORED,
    PRIMARY KEY (product_id, attribute_id, value)
);
CREATE INDEX idx_prodattr_attr ON product_attributes(attribute_id, value_norm);

-- تریگر به‌روزرسانی updated_at.
CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS trigger AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_products_touch BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

//! Repository مدیریت کاتالوگ — CRUD محصول، تنوع و تصویر (پنل ادمین).
//! خواندن‌های فروشگاه در `catalog.rs` است؛ این فایل مخصوص نوشتن/مدیریت است.

use crate::models::{Brand, Category, Product, ProductImage, ProductVariant};
use crate::Db;
use uuid::Uuid;

/// ردیف فهرست مدیریت محصول — شامل غیرفعال‌ها، با نام برند/دسته و شمار تنوع/موجودی.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AdminProductRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub base_price: i64,
    pub is_active: bool,
    pub is_featured: bool,
    pub brand_name: Option<String>,
    pub category_name: Option<String>,
    pub variant_count: i64,
    pub total_stock: Option<i64>,
}

impl AdminProductRow {
    pub fn price_fa(&self) -> String {
        domain::Rial(self.base_price).format_toman_fa()
    }
}

/// فهرست محصولات (فعال و غیرفعال) برای پنل ادمین، با جستجوی اختیاری روی نام/نامک.
pub async fn list_all_products(
    db: &Db,
    search: Option<&str>,
) -> Result<Vec<AdminProductRow>, sqlx::Error> {
    let base = r#"
        SELECT p.id, p.name, p.slug, p.base_price, p.is_active, p.is_featured,
               b.name AS brand_name, c.name AS category_name,
               (SELECT count(*) FROM product_variants v WHERE v.product_id = p.id) AS variant_count,
               (SELECT sum(v.stock)::bigint FROM product_variants v WHERE v.product_id = p.id) AS total_stock
        FROM products p
        LEFT JOIN brands b ON b.id = p.brand_id
        LEFT JOIN categories c ON c.id = p.category_id
    "#;
    match search.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(q) => {
            sqlx::query_as::<_, AdminProductRow>(&format!(
                "{base} WHERE p.name ILIKE $1 OR p.slug ILIKE $1 OR p.name_norm % fa_normalize($2)
                 ORDER BY p.created_at DESC LIMIT 300"
            ))
            .bind(format!("%{q}%"))
            .bind(q)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, AdminProductRow>(&format!("{base} ORDER BY p.created_at DESC LIMIT 300"))
                .fetch_all(db)
                .await
        }
    }
}

/// آیا این slug قبلاً استفاده شده؟ (در صورت ویرایش، خودِ محصول را نادیده می‌گیرد.)
pub async fn slug_exists(
    db: &Db,
    slug: &str,
    exclude_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM products WHERE slug = $1 AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(slug)
    .bind(exclude_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

/// داده‌های ورودی برای ساخت/ویرایش محصول.
pub struct ProductInput {
    pub name: String,
    pub slug: String,
    pub category_id: Option<Uuid>,
    pub brand_id: Option<Uuid>,
    pub short_desc: Option<String>,
    pub description: Option<String>,
    pub base_price: i64, // ریال
    pub is_active: bool,
    pub is_featured: bool,
    pub video_url: Option<String>,
    pub deal_ends_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn create_product(db: &Db, p: &ProductInput) -> Result<Product, sqlx::Error> {
    sqlx::query_as::<_, Product>(
        r#"INSERT INTO products
           (name, slug, category_id, brand_id, short_desc, description, base_price, is_active, is_featured, video_url, deal_ends_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *"#,
    )
    .bind(&p.name)
    .bind(&p.slug)
    .bind(p.category_id)
    .bind(p.brand_id)
    .bind(&p.short_desc)
    .bind(&p.description)
    .bind(p.base_price)
    .bind(p.is_active)
    .bind(p.is_featured)
    .bind(&p.video_url)
    .bind(p.deal_ends_at)
    .fetch_one(db)
    .await
}

pub async fn update_product(db: &Db, id: Uuid, p: &ProductInput) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE products SET
           name=$2, slug=$3, category_id=$4, brand_id=$5,
           short_desc=$6, description=$7, base_price=$8, is_active=$9, is_featured=$10, video_url=$11, deal_ends_at=$12
           WHERE id=$1"#,
    )
    .bind(id)
    .bind(&p.name)
    .bind(&p.slug)
    .bind(p.category_id)
    .bind(p.brand_id)
    .bind(&p.short_desc)
    .bind(&p.description)
    .bind(p.base_price)
    .bind(p.is_active)
    .bind(p.is_featured)
    .bind(&p.video_url)
    .bind(p.deal_ends_at)
    .execute(db)
    .await?;
    Ok(())
}

/// حذف کامل محصول (تنوع‌ها و تصاویر با ON DELETE CASCADE پاک می‌شوند).
/// تاریخچه‌ی سفارش حفظ می‌شود (order_items snapshot دارد و variant_id با SET NULL آزاد می‌شود).
pub async fn delete_product(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_active(db: &Db, id: Uuid, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE products SET is_active = $2 WHERE id = $1")
        .bind(id)
        .bind(active)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_featured(db: &Db, id: Uuid, featured: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE products SET is_featured = $2 WHERE id = $1")
        .bind(id)
        .bind(featured)
        .execute(db)
        .await?;
    Ok(())
}

/// محصول خام بر اساس id (بدون شرط is_active — برای پنل ادمین).
pub async fn find_product(db: &Db, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
    sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

// ---------- تنوع‌ها ----------

/// همه‌ی تنوع‌های یک محصول (شامل غیرفعال‌ها).
pub async fn product_variants(db: &Db, product_id: Uuid) -> Result<Vec<ProductVariant>, sqlx::Error> {
    sqlx::query_as::<_, ProductVariant>(
        "SELECT * FROM product_variants WHERE product_id = $1 ORDER BY sort_order, price",
    )
    .bind(product_id)
    .fetch_all(db)
    .await
}

pub struct VariantInput {
    pub title: String,
    pub sku: Option<String>,
    pub price: i64, // ریال
    pub compare_at_price: Option<i64>, // ریال؛ قیمت قبل از تخفیف
    pub stock: i32,
    pub is_active: bool,
    pub sort_order: i32,
}

pub async fn add_variant(
    db: &Db,
    product_id: Uuid,
    v: &VariantInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO product_variants (product_id, title, sku, price, compare_at_price, stock, is_active, sort_order)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(product_id)
    .bind(&v.title)
    .bind(&v.sku)
    .bind(v.price)
    .bind(v.compare_at_price)
    .bind(v.stock)
    .bind(v.is_active)
    .bind(v.sort_order)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_variant(db: &Db, id: Uuid, v: &VariantInput) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE product_variants SET title=$2, sku=$3, price=$4, compare_at_price=$5, stock=$6, is_active=$7, sort_order=$8
         WHERE id=$1",
    )
    .bind(id)
    .bind(&v.title)
    .bind(&v.sku)
    .bind(v.price)
    .bind(v.compare_at_price)
    .bind(v.stock)
    .bind(v.is_active)
    .bind(v.sort_order)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_variant(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM product_variants WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// وضعیت فعلی تنوع برای مقایسه‌ی قبل/بعد (قیمت، موجودی) + محصولِ والد.
/// خروجی: (price, stock, product_id, product_name, product_slug).
pub async fn variant_snapshot(
    db: &Db,
    id: Uuid,
) -> Result<Option<(i64, i32, Uuid, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i32, Uuid, String, String)>(
        "SELECT v.price, v.stock, p.id, p.name, p.slug
         FROM product_variants v JOIN products p ON p.id = v.product_id
         WHERE v.id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

// ---------- تصاویر ----------

pub async fn product_images(db: &Db, product_id: Uuid) -> Result<Vec<ProductImage>, sqlx::Error> {
    sqlx::query_as::<_, ProductImage>(
        "SELECT * FROM product_images WHERE product_id = $1 ORDER BY is_primary DESC, sort_order",
    )
    .bind(product_id)
    .fetch_all(db)
    .await
}

/// افزودن تصویر با مسیر/URL (تا قبل از خط لوله‌ی آپلود WebP/AVIF، مسیر دستی).
pub async fn add_image(
    db: &Db,
    product_id: Uuid,
    path: &str,
    alt: Option<&str>,
) -> Result<(), sqlx::Error> {
    // اگر نخستین تصویر محصول است، خودبه‌خود primary شود.
    let mut tx = db.begin().await?;
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM product_images WHERE product_id = $1",
    )
    .bind(product_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO product_images (product_id, path, alt, sort_order, is_primary)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(product_id)
    .bind(path)
    .bind(alt)
    .bind(count as i32)
    .bind(count == 0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// حذف یک تصویر. اگر primary بود، تصویر دیگری (در صورت وجود) primary می‌شود.
pub async fn delete_image(db: &Db, image_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, bool)>(
        "DELETE FROM product_images WHERE id = $1 RETURNING product_id, is_primary",
    )
    .bind(image_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((product_id, was_primary)) = row {
        if was_primary {
            // نخستین تصویر باقی‌مانده را primary کن.
            sqlx::query(
                "UPDATE product_images SET is_primary = true
                 WHERE id = (SELECT id FROM product_images WHERE product_id = $1
                             ORDER BY sort_order LIMIT 1)",
            )
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// تعیین تصویر اصلی (primary) — بقیه‌ی تصاویر همان محصول غیر‌primary می‌شوند.
pub async fn set_primary_image(db: &Db, image_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    let product_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT product_id FROM product_images WHERE id = $1",
    )
    .bind(image_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(pid) = product_id {
        sqlx::query("UPDATE product_images SET is_primary = (id = $1) WHERE product_id = $2")
            .bind(image_id)
            .bind(pid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ---------- دسته/برند برای منوهای فرم ----------

/// همه‌ی دسته‌ها (شامل غیرفعال) برای منوی کشویی فرم.
pub async fn all_categories(db: &Db) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(
        "SELECT id, parent_id, name, slug, description, icon, sort_order, is_active
         FROM categories ORDER BY sort_order, name",
    )
    .fetch_all(db)
    .await
}

pub async fn all_brands(db: &Db) -> Result<Vec<Brand>, sqlx::Error> {
    sqlx::query_as::<_, Brand>("SELECT id, name, slug, logo FROM brands ORDER BY name")
        .fetch_all(db)
        .await
}

// ===================== مدیریت دسته‌بندی =====================

/// ردیف فهرست مدیریت دسته — با نام والد و شمار محصولات.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AdminCategoryRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub parent_name: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub product_count: i64,
}

pub async fn list_categories_admin(db: &Db) -> Result<Vec<AdminCategoryRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminCategoryRow>(
        r#"
        SELECT c.id, c.name, c.slug, c.parent_id, p.name AS parent_name,
               c.sort_order, c.is_active,
               (SELECT count(*) FROM products pr WHERE pr.category_id = c.id) AS product_count
        FROM categories c
        LEFT JOIN categories p ON p.id = c.parent_id
        ORDER BY c.sort_order, c.name
        "#,
    )
    .fetch_all(db)
    .await
}

pub async fn find_category(db: &Db, id: Uuid) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(
        "SELECT id, parent_id, name, slug, description, icon, sort_order, is_active
         FROM categories WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn category_slug_exists(
    db: &Db,
    slug: &str,
    exclude_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM categories WHERE slug = $1 AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(slug)
    .bind(exclude_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

pub struct CategoryInput {
    pub name: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

pub async fn create_category(db: &Db, c: &CategoryInput) -> Result<Category, sqlx::Error> {
    sqlx::query_as::<_, Category>(
        r#"INSERT INTO categories (name, slug, parent_id, description, icon, sort_order, is_active)
           VALUES ($1,$2,$3,$4,$5,$6,$7)
           RETURNING id, parent_id, name, slug, description, icon, sort_order, is_active"#,
    )
    .bind(&c.name)
    .bind(&c.slug)
    .bind(c.parent_id)
    .bind(&c.description)
    .bind(&c.icon)
    .bind(c.sort_order)
    .bind(c.is_active)
    .fetch_one(db)
    .await
}

pub async fn update_category(db: &Db, id: Uuid, c: &CategoryInput) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE categories SET
           name=$2, slug=$3, parent_id=$4, description=$5, icon=$6, sort_order=$7, is_active=$8
           WHERE id=$1"#,
    )
    .bind(id)
    .bind(&c.name)
    .bind(&c.slug)
    .bind(c.parent_id)
    .bind(&c.description)
    .bind(&c.icon)
    .bind(c.sort_order)
    .bind(c.is_active)
    .execute(db)
    .await?;
    Ok(())
}

/// حذف دسته. محصولات و زیردسته‌ها با ON DELETE SET NULL والد/دسته‌شان خالی می‌شود (حذف نمی‌شوند).
pub async fn delete_category(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_category_active(db: &Db, id: Uuid, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE categories SET is_active = $2 WHERE id = $1")
        .bind(id)
        .bind(active)
        .execute(db)
        .await?;
    Ok(())
}

// ===================== مدیریت برند =====================

/// ردیف فهرست مدیریت برند — با شمار محصولات.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AdminBrandRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub logo: Option<String>,
    pub product_count: i64,
}

pub async fn list_brands_admin(db: &Db) -> Result<Vec<AdminBrandRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminBrandRow>(
        r#"
        SELECT b.id, b.name, b.slug, b.logo,
               (SELECT count(*) FROM products pr WHERE pr.brand_id = b.id) AS product_count
        FROM brands b
        ORDER BY b.name
        "#,
    )
    .fetch_all(db)
    .await
}

pub async fn find_brand(db: &Db, id: Uuid) -> Result<Option<Brand>, sqlx::Error> {
    sqlx::query_as::<_, Brand>("SELECT id, name, slug, logo FROM brands WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn brand_slug_exists(
    db: &Db,
    slug: &str,
    exclude_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM brands WHERE slug = $1 AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(slug)
    .bind(exclude_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

pub struct BrandInput {
    pub name: String,
    pub slug: String,
    pub logo: Option<String>,
}

pub async fn create_brand(db: &Db, b: &BrandInput) -> Result<Brand, sqlx::Error> {
    sqlx::query_as::<_, Brand>(
        "INSERT INTO brands (name, slug, logo) VALUES ($1,$2,$3)
         RETURNING id, name, slug, logo",
    )
    .bind(&b.name)
    .bind(&b.slug)
    .bind(&b.logo)
    .fetch_one(db)
    .await
}

pub async fn update_brand(db: &Db, id: Uuid, b: &BrandInput) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE brands SET name=$2, slug=$3, logo=$4 WHERE id=$1")
        .bind(id)
        .bind(&b.name)
        .bind(&b.slug)
        .bind(&b.logo)
        .execute(db)
        .await?;
    Ok(())
}

/// حذف برند. محصولات این برند با ON DELETE SET NULL بی‌برند می‌شوند (حذف نمی‌شوند).
pub async fn delete_brand(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM brands WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

//! Repository ویژگی‌های محصول (مشخصات فنی + فیلتر پویا) — فاز ارتقا.
//! جدول‌های `attributes` و `product_attributes` از migration 0002 موجودند.

use crate::Db;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// یک جفت ویژگی-مقدار برای نمایش در جدول مشخصات محصول.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AttrValue {
    pub name: String,
    pub value: String,
}

/// ویژگی‌های یک محصول (برای جدول مشخصات).
pub async fn for_product(db: &Db, product_id: Uuid) -> Result<Vec<AttrValue>, sqlx::Error> {
    sqlx::query_as::<_, AttrValue>(
        "SELECT a.name, pa.value
         FROM product_attributes pa
         JOIN attributes a ON a.id = pa.attribute_id
         WHERE pa.product_id = $1
         ORDER BY a.sort_order, a.name",
    )
    .bind(product_id)
    .fetch_all(db)
    .await
}

/// یک ویژگی قابل‌فیلتر به‌همراه مقادیر متمایز موجود (برای نوار فیلتر کاتالوگ).
#[derive(Debug, Clone, Serialize)]
pub struct FilterableAttr {
    pub slug: String,
    pub name: String,
    pub values: Vec<String>,
}

/// ویژگی‌های قابل‌فیلتر و مقادیر متمایزشان (محدود به محصولات فعال).
pub async fn filterable(db: &Db) -> Result<Vec<FilterableAttr>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT DISTINCT a.slug, a.name, pa.value
         FROM attributes a
         JOIN product_attributes pa ON pa.attribute_id = a.id
         JOIN products p ON p.id = pa.product_id AND p.is_active
         WHERE a.is_filterable
         ORDER BY a.slug, pa.value",
    )
    .fetch_all(db)
    .await?;

    // گروه‌بندی مقادیر زیر هر ویژگی (با حفظ ترتیب).
    let mut out: Vec<FilterableAttr> = Vec::new();
    for (slug, name, value) in rows {
        match out.last_mut() {
            Some(last) if last.slug == slug => last.values.push(value),
            _ => out.push(FilterableAttr {
                slug,
                name,
                values: vec![value],
            }),
        }
    }
    Ok(out)
}

// ---------- مدیریت (ادمین) ----------

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Attribute {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub is_filterable: bool,
    pub sort_order: i32,
}

pub async fn all_attributes(db: &Db) -> Result<Vec<Attribute>, sqlx::Error> {
    sqlx::query_as::<_, Attribute>(
        "SELECT id, name, slug, is_filterable, sort_order FROM attributes ORDER BY sort_order, name",
    )
    .fetch_all(db)
    .await
}

/// مقادیر ویژگی یک محصول با id ویژگی (برای فرم ادمین).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ProductAttrRow {
    pub attribute_id: Uuid,
    pub name: String,
    pub value: String,
}

pub async fn product_attr_rows(db: &Db, product_id: Uuid) -> Result<Vec<ProductAttrRow>, sqlx::Error> {
    sqlx::query_as::<_, ProductAttrRow>(
        "SELECT pa.attribute_id, a.name, pa.value
         FROM product_attributes pa JOIN attributes a ON a.id = pa.attribute_id
         WHERE pa.product_id = $1 ORDER BY a.sort_order",
    )
    .bind(product_id)
    .fetch_all(db)
    .await
}

/// تنظیم (یا حذف) مقدار یک ویژگی برای محصول. مقدار خالی = حذف.
pub async fn set_value(
    db: &Db,
    product_id: Uuid,
    attribute_id: Uuid,
    value: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM product_attributes WHERE product_id = $1 AND attribute_id = $2")
        .bind(product_id)
        .bind(attribute_id)
        .execute(&mut *tx)
        .await?;
    if !value.trim().is_empty() {
        sqlx::query(
            "INSERT INTO product_attributes (product_id, attribute_id, value) VALUES ($1, $2, $3)",
        )
        .bind(product_id)
        .bind(attribute_id)
        .bind(value.trim())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn create_attribute(db: &Db, name: &str, slug: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attributes (name, slug, is_filterable, sort_order)
         VALUES ($1, $2, true, COALESCE((SELECT max(sort_order)+1 FROM attributes), 0))
         ON CONFLICT (slug) DO NOTHING",
    )
    .bind(name)
    .bind(slug)
    .execute(db)
    .await?;
    Ok(())
}

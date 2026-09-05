//! Repository کاتالوگ — دسته‌بندی، محصول، فیلتر و جستجو.

use crate::models::*;
use crate::Db;
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

/// پارامترهای فهرست/فیلتر محصولات.
#[derive(Debug, Default, Clone)]
pub struct ProductQuery {
    pub category_slug: Option<String>,
    pub brand_slug: Option<String>,
    pub search: Option<String>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    /// فیلتر ویژگی‌ها: (slug ویژگی → مقادیر انتخاب‌شده). درون هر ویژگی OR، بین ویژگی‌ها AND.
    pub attrs: Vec<(String, Vec<String>)>,
    /// فقط محصولات موجود.
    pub in_stock_only: bool,
    /// فقط محصولات تخفیف‌دار.
    pub discounted_only: bool,
    /// حداقل میانگین امتیاز (۰ = بدون فیلتر).
    pub min_rating: i64,
    pub sort: ProductSort,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProductSort {
    #[default]
    Newest,
    PriceAsc,
    PriceDesc,
    Featured,
    BestSelling,
    Discount,
}

impl ProductSort {
    pub fn parse(s: &str) -> Self {
        match s {
            "price_asc" => Self::PriceAsc,
            "price_desc" => Self::PriceDesc,
            "featured" => Self::Featured,
            "bestselling" => Self::BestSelling,
            "discount" => Self::Discount,
            _ => Self::Newest,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Newest => "newest",
            Self::PriceAsc => "price_asc",
            Self::PriceDesc => "price_desc",
            Self::Featured => "featured",
            Self::BestSelling => "bestselling",
            Self::Discount => "discount",
        }
    }
}

#[derive(Debug)]
pub struct PagedProducts {
    pub items: Vec<ProductCard>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl PagedProducts {
    pub fn total_pages(&self) -> i64 {
        if self.per_page <= 0 {
            return 1;
        }
        ((self.total + self.per_page - 1) / self.per_page).max(1)
    }
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages()
    }
}

/// فهرست همه‌ی دسته‌بندی‌های فعال.
pub async fn list_categories(db: &Db) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(
        "SELECT id, parent_id, name, slug, description, icon, sort_order, is_active
         FROM categories WHERE is_active ORDER BY sort_order, name",
    )
    .fetch_all(db)
    .await
}

/// فهرست برندها.
pub async fn list_brands(db: &Db) -> Result<Vec<Brand>, sqlx::Error> {
    sqlx::query_as::<_, Brand>("SELECT id, name, slug, logo FROM brands ORDER BY name")
        .fetch_all(db)
        .await
}

const CARD_SELECT: &str = r#"
    SELECT p.id, p.name, p.slug, p.short_desc, p.base_price,
           (SELECT min(v.price) FROM product_variants v
            WHERE v.product_id = p.id AND v.is_active) AS min_price,
           (SELECT v.compare_at_price FROM product_variants v
            WHERE v.product_id = p.id AND v.is_active
            ORDER BY v.price LIMIT 1) AS compare_at,
           b.name AS brand_name, c.slug AS category_slug,
           (SELECT path FROM product_images pi WHERE pi.product_id = p.id
            ORDER BY pi.is_primary DESC, pi.sort_order LIMIT 1) AS primary_image,
           EXISTS (SELECT 1 FROM product_variants v
                   WHERE v.product_id = p.id AND v.is_active AND v.stock > 0) AS in_stock,
           COALESCE((SELECT avg(rating) FROM product_reviews r
                     WHERE r.product_id = p.id AND r.status = 'approved'), 0)::float8 AS rating_avg,
           (SELECT count(*) FROM product_reviews r
            WHERE r.product_id = p.id AND r.status = 'approved') AS rating_count
    FROM products p
    LEFT JOIN brands b ON b.id = p.brand_id
    LEFT JOIN categories c ON c.id = p.category_id
"#;

/// پیشنهاد جستجوی زنده (autocomplete) — تطبیق فازی فارسی روی نام نرمال‌شده.
pub async fn search_suggest(db: &Db, term: &str, limit: i64) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!(
        "{CARD_SELECT} WHERE p.is_active AND
         (p.name_norm ILIKE ('%' || fa_normalize($1) || '%') OR p.name_norm % fa_normalize($1))
         ORDER BY similarity(p.name_norm, fa_normalize($1)) DESC LIMIT $2"
    );
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(term)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// کارت‌های چند محصول بر اساس شناسه (برای «بازدیدهای اخیر»). ترتیب ورودی حفظ نمی‌شود.
pub async fn cards_by_ids(db: &Db, ids: &[Uuid]) -> Result<Vec<ProductCard>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!("{CARD_SELECT} WHERE p.is_active AND p.id = ANY($1)");
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(ids)
        .fetch_all(db)
        .await
}

/// محصولات مرتبط: هم‌دسته با محصول جاری (به‌جز خودش).
pub async fn related_products(
    db: &Db,
    product_id: Uuid,
    category_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!(
        "{CARD_SELECT} WHERE p.is_active AND p.id <> $1
         AND ($2::uuid IS NULL OR p.category_id = $2)
         ORDER BY p.is_featured DESC, p.view_count DESC LIMIT $3"
    );
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(product_id)
        .bind(category_id)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// پربازدیدترین محصولات فعال (پرفروش/محبوب).
pub async fn most_viewed(db: &Db, limit: i64) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!(
        "{CARD_SELECT} WHERE p.is_active ORDER BY p.view_count DESC, p.created_at DESC LIMIT $1"
    );
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// جدیدترین محصولات فعال.
pub async fn newest_products(db: &Db, limit: i64) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!("{CARD_SELECT} WHERE p.is_active ORDER BY p.created_at DESC LIMIT $1");
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// محصولات دارای پیشنهاد لحظه‌ای فعال (تایمر نگذشته) — برای ریل «شگفت‌انگیز».
pub async fn deal_products(db: &Db, limit: i64) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!(
        "{CARD_SELECT} WHERE p.is_active AND p.deal_ends_at IS NOT NULL AND p.deal_ends_at > now()
         ORDER BY p.deal_ends_at ASC LIMIT $1"
    );
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// محصولات شاخص برای صفحه‌ی اصلی.
pub async fn featured_products(db: &Db, limit: i64) -> Result<Vec<ProductCard>, sqlx::Error> {
    let sql = format!(
        "{CARD_SELECT} WHERE p.is_active AND p.is_featured ORDER BY p.created_at DESC LIMIT $1"
    );
    sqlx::query_as::<_, ProductCard>(&sql)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// اعمال شرط‌های WHERE روی یک QueryBuilder (مشترک بین count و list).
fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, q: &'a ProductQuery, search: &'a Option<String>) {
    qb.push(" WHERE p.is_active");
    if let Some(cat) = &q.category_slug {
        qb.push(" AND (c.slug = ");
        qb.push_bind(cat);
        qb.push(" OR c.parent_id = (SELECT id FROM categories WHERE slug = ");
        qb.push_bind(cat);
        qb.push("))");
    }
    if let Some(brand) = &q.brand_slug {
        qb.push(" AND b.slug = ");
        qb.push_bind(brand);
    }
    if let Some(s) = search {
        // تطبیق زیررشته‌ای (ILIKE روی نام نرمال‌شده) + تطبیق فازی trigram (برای غلط‌های تایپی).
        qb.push(" AND (p.name_norm ILIKE ('%' || fa_normalize(");
        qb.push_bind(s);
        qb.push(") || '%') OR p.name_norm % fa_normalize(");
        qb.push_bind(s);
        qb.push("))");
    }
    if let Some(mn) = q.min_price {
        qb.push(" AND p.base_price >= ");
        qb.push_bind(mn);
    }
    if let Some(mx) = q.max_price {
        qb.push(" AND p.base_price <= ");
        qb.push_bind(mx);
    }
    if q.in_stock_only {
        qb.push(" AND EXISTS (SELECT 1 FROM product_variants v WHERE v.product_id = p.id AND v.is_active AND v.stock > 0)");
    }
    if q.discounted_only {
        qb.push(" AND EXISTS (SELECT 1 FROM product_variants v WHERE v.product_id = p.id AND v.is_active AND v.compare_at_price > v.price)");
    }
    if q.min_rating > 0 {
        qb.push(" AND (SELECT COALESCE(avg(rating),0) FROM product_reviews r WHERE r.product_id = p.id AND r.status = 'approved') >= ");
        qb.push_bind(q.min_rating as f64);
    }
    // فیلتر ویژگی‌ها: برای هر ویژگی، محصول باید یکی از مقادیر انتخاب‌شده را داشته باشد.
    for (slug, values) in &q.attrs {
        if values.is_empty() {
            continue;
        }
        qb.push(
            " AND EXISTS (SELECT 1 FROM product_attributes pa
                          JOIN attributes a ON a.id = pa.attribute_id
                          WHERE pa.product_id = p.id AND a.slug = ",
        );
        qb.push_bind(slug);
        qb.push(" AND pa.value = ANY(");
        qb.push_bind(values);
        qb.push("))");
    }
}

/// فهرست محصولات با فیلتر، جستجوی فارسی و صفحه‌بندی.
pub async fn list_products(db: &Db, q: &ProductQuery) -> Result<PagedProducts, sqlx::Error> {
    let per_page = if q.per_page <= 0 { 12 } else { q.per_page.clamp(1, 60) };
    let page = q.page.max(1);
    let offset = (page - 1) * per_page;
    let search = q
        .search
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| s.chars().count() >= 2);

    // --- شمارش کل ---
    let mut cb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT count(*) FROM products p
         LEFT JOIN brands b ON b.id = p.brand_id
         LEFT JOIN categories c ON c.id = p.category_id",
    );
    apply_filters(&mut cb, q, &search);
    let total: i64 = cb.build_query_scalar().fetch_one(db).await?;

    // --- فهرست ---
    let mut lb: QueryBuilder<Postgres> = QueryBuilder::new(CARD_SELECT);
    apply_filters(&mut lb, q, &search);
    if let Some(s) = &search {
        lb.push(" ORDER BY similarity(p.name_norm, fa_normalize(");
        lb.push_bind(s);
        lb.push(")) DESC");
    } else {
        lb.push(" ORDER BY ");
        lb.push(q.sort.order_clause());
    }
    lb.push(" LIMIT ");
    lb.push_bind(per_page);
    lb.push(" OFFSET ");
    lb.push_bind(offset);

    let items = lb.build_query_as::<ProductCard>().fetch_all(db).await?;

    Ok(PagedProducts {
        items,
        total,
        page,
        per_page,
    })
}

impl ProductSort {
    fn order_clause(&self) -> &'static str {
        match self {
            Self::Newest => "p.created_at DESC",
            Self::PriceAsc => "p.base_price ASC",
            Self::PriceDesc => "p.base_price DESC",
            Self::Featured => "p.is_featured DESC, p.created_at DESC",
            Self::BestSelling => "p.view_count DESC, p.created_at DESC",
            Self::Discount => "(SELECT max(v.compare_at_price - v.price) FROM product_variants v \
                                WHERE v.product_id = p.id AND v.is_active AND v.compare_at_price > v.price) \
                               DESC NULLS LAST, p.created_at DESC",
        }
    }
}

/// یک محصول کامل با تنوع‌ها و تصاویر (برای صفحه‌ی محصول).
pub struct ProductDetail {
    pub product: Product,
    pub brand: Option<Brand>,
    pub category: Option<Category>,
    pub variants: Vec<ProductVariant>,
    pub images: Vec<ProductImage>,
}

pub async fn product_by_slug(db: &Db, slug: &str) -> Result<Option<ProductDetail>, sqlx::Error> {
    let product =
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE slug = $1 AND is_active")
            .bind(slug)
            .fetch_optional(db)
            .await?;

    let Some(product) = product else {
        return Ok(None);
    };

    let brand = if let Some(bid) = product.brand_id {
        sqlx::query_as::<_, Brand>("SELECT id, name, slug, logo FROM brands WHERE id = $1")
            .bind(bid)
            .fetch_optional(db)
            .await?
    } else {
        None
    };

    let category = if let Some(cid) = product.category_id {
        sqlx::query_as::<_, Category>(
            "SELECT id, parent_id, name, slug, description, icon, sort_order, is_active
             FROM categories WHERE id = $1",
        )
        .bind(cid)
        .fetch_optional(db)
        .await?
    } else {
        None
    };

    let variants = sqlx::query_as::<_, ProductVariant>(
        "SELECT * FROM product_variants WHERE product_id = $1 AND is_active ORDER BY sort_order, price",
    )
    .bind(product.id)
    .fetch_all(db)
    .await?;

    let images = sqlx::query_as::<_, ProductImage>(
        "SELECT * FROM product_images WHERE product_id = $1 ORDER BY is_primary DESC, sort_order",
    )
    .bind(product.id)
    .fetch_all(db)
    .await?;

    Ok(Some(ProductDetail {
        product,
        brand,
        category,
        variants,
        images,
    }))
}

/// همه‌ی slugهای محصولات فعال (برای sitemap).
pub async fn all_active_slugs(db: &Db) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT slug FROM products WHERE is_active ORDER BY created_at DESC")
        .fetch_all(db)
        .await
}

/// slug + زمان آخرین تغییر هر محصول فعال — برای `lastmod` در sitemap.
pub async fn active_slugs_with_lastmod(
    db: &Db,
) -> Result<Vec<(String, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
    sqlx::query_as::<_, (String, chrono::DateTime<chrono::Utc>)>(
        "SELECT slug, updated_at FROM products WHERE is_active ORDER BY updated_at DESC",
    )
    .fetch_all(db)
    .await
}

/// افزایش شمارنده‌ی بازدید.
pub async fn increment_view(db: &Db, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE products SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

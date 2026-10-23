//! روت‌های مدیریت سفارش (پنل ادمین): فهرست با فیلتر، جزئیات، تغییر وضعیت.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use db::orders::{Order, ORDER_STATUSES};
use db::orders_admin::{self, AdminOrderRow, OrderItem, PaymentRow};
use serde::Deserialize;
use uuid::Uuid;

/// یک گزینه‌ی وضعیت با برچسب فارسی (برای منو/تب).
struct StatusOption {
    key: &'static str,
    label: &'static str,
}

fn status_options() -> Vec<StatusOption> {
    ORDER_STATUSES
        .iter()
        .map(|k| StatusOption {
            key: k,
            label: db::orders::status_label(k),
        })
        .collect()
}

// ===================== فهرست =====================

#[derive(Template)]
#[template(path = "admin/orders.html")]
struct OrdersTemplate {
    site_name: String,
    orders: Vec<AdminOrderRow>,
    statuses: Vec<StatusOption>,
    active_filter: String,
    total_count: i64,
}

#[derive(Deserialize)]
pub struct OrdersQuery {
    pub status: Option<String>,
}

pub async fn list(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<OrdersQuery>,
) -> AppResult<Html<String>> {
    let filter = q.status.filter(|s| ORDER_STATUSES.contains(&s.as_str()));
    let orders = orders_admin::list(&s.db, filter.as_deref()).await?;
    let total_count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM orders")
        .fetch_one(&s.db)
        .await?;
    render(OrdersTemplate {
        site_name: s.site_name(),
        total_count,
        statuses: status_options(),
        active_filter: filter.unwrap_or_default(),
        orders,
    })
}

// ===================== جزئیات =====================

#[derive(Template)]
#[template(path = "admin/order_detail.html")]
struct OrderDetailTemplate {
    site_name: String,
    order: Order,
    order_number_fa: String,
    items: Vec<OrderItem>,
    payments: Vec<PaymentRow>,
    customer_mobile: Option<String>,
    statuses: Vec<StatusOption>,
}

pub async fn detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let d = orders_admin::detail(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("سفارش".into()))?;
    let order_number_fa = crate::filters::to_fa_digits(&d.order.order_number.to_string());
    render(OrderDetailTemplate {
        site_name: s.site_name(),
        order_number_fa,
        items: d.items,
        payments: d.payments,
        customer_mobile: d.customer_mobile,
        statuses: status_options(),
        order: d.order,
    })
}

// ===================== تغییر وضعیت =====================

#[derive(Deserialize)]
pub struct UpdateOrderForm {
    pub status: String,
}

pub async fn update_status(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<UpdateOrderForm>,
) -> AppResult<Response> {
    if !ORDER_STATUSES.contains(&f.status.as_str()) {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "وضعیت نامعتبر است.".into(),
        )));
    }
    if f.status == "cancelled" {
        // لغو: موجودی به انبار برمی‌گردد و امتیاز/کیف‌پولِ خرج‌شده بازگردانده می‌شود.
        if db::orders::cancel_and_restock(&s.db, id).await? {
            if let Err(e) = db::loyalty::reverse_purchase_points(&s.db, id).await {
                tracing::error!(error = %e, "بازگردانی امتیاز هنگام لغو شکست خورد");
            }
            let _ = db::wallet::reverse_order_spend(&s.db, id).await;
        }
    } else {
        db::orders::set_status(&s.db, id, &f.status).await?;
    }
    // اعلان تغییر وضعیت برای مشتری.
    if let Ok(Some(order)) = db::orders::find(&s.db, id).await {
        if let Some(uid) = order.user_id {
            let num = crate::filters::to_fa_digits(&order.order_number.to_string());
            let title = format!("سفارش #{num}: {}", db::orders::status_label(&f.status));
            let url = format!("/order/{id}");
            let _ = db::notifications::create(&s.db, uid, "order", &title, None, Some(&url)).await;
        }
    }
    Ok(Redirect::to(&format!("/admin/orders/{id}")).into_response())
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

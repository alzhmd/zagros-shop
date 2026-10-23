//! روت‌های مدیریت کاربران (پنل ادمین): فهرست/جستجو، جزئیات، فعال/مسدود،
//! تغییر نقش، و تنظیم دستی امتیاز.

use crate::error::{AppError, AppResult};
use crate::extract::AdminUser;
use crate::routes::admin_util::{checkbox, parse_digits};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use db::accounts::User;
use db::loyalty::LoyaltyTx;
use db::orders::Order;
use db::repairs::RepairRequest;
use db::users_admin::{self, AdminUserRow};
use serde::Deserialize;
use uuid::Uuid;

const ROLES: [&str; 4] = ["customer", "operator", "technician", "admin"];

// ===================== فهرست =====================

#[derive(Template)]
#[template(path = "admin/users.html")]
struct UsersTemplate {
    site_name: String,
    users: Vec<AdminUserRow>,
    total_count: i64,
    search: String,
}

#[derive(Deserialize)]
pub struct UsersQuery {
    pub q: Option<String>,
}

pub async fn list(
    State(s): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<UsersQuery>,
) -> AppResult<Html<String>> {
    let users = users_admin::list(&s.db, q.q.as_deref()).await?;
    let total_count = users_admin::count(&s.db).await?;
    render(UsersTemplate {
        site_name: s.site_name(),
        users,
        total_count,
        search: q.q.unwrap_or_default(),
    })
}

// ===================== جزئیات =====================

struct RoleOption {
    key: &'static str,
    label: &'static str,
}

#[derive(Template)]
#[template(path = "admin/user_detail.html")]
struct UserDetailTemplate {
    site_name: String,
    user: User,
    points_fa: String,
    wallet_fa: String,
    order_count: i64,
    repair_count: i64,
    loyalty: Vec<LoyaltyTx>,
    orders: Vec<Order>,
    repairs: Vec<RepairRequest>,
    roles: Vec<RoleOption>,
    joined_fa: String,
}

pub async fn detail(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let user = users_admin::find(&s.db, id)
        .await?
        .ok_or_else(|| domain::DomainError::NotFound("کاربر".into()))?;
    render(build_user_detail(&s, user).await?)
}

async fn build_user_detail(s: &AppState, user: User) -> AppResult<UserDetailTemplate> {
    let (order_count, repair_count) = users_admin::user_stats(&s.db, user.id).await?;
    let loyalty = db::loyalty::history(&s.db, user.id, 15).await?;
    let orders = db::orders::list_for_user(&s.db, user.id).await?;
    let repairs = db::repairs::list_for_user(&s.db, user.id).await?;
    let points_fa = crate::filters::to_fa_digits(&user.points.to_string());
    let joined_fa = crate::filters::to_fa_digits(&user.created_at.format("%Y-%m-%d").to_string());
    let roles = ROLES
        .iter()
        .map(|k| RoleOption {
            key: k,
            label: users_admin::role_label(k),
        })
        .collect();
    Ok(UserDetailTemplate {
        site_name: s.site_name(),
        points_fa,
        wallet_fa: domain::Rial(user.wallet_balance).format_toman_fa(),
        order_count,
        repair_count,
        loyalty,
        orders,
        repairs,
        roles,
        joined_fa,
        user,
    })
}

// ===================== فعال/مسدود + نقش =====================

#[derive(Deserialize)]
pub struct ToggleForm {
    pub value: Option<String>,
}

pub async fn toggle_active(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<ToggleForm>,
) -> AppResult<Response> {
    users_admin::set_active(&s.db, id, checkbox(&f.value)).await?;
    Ok(Redirect::to(&format!("/admin/users/{id}")).into_response())
}

#[derive(Deserialize)]
pub struct RoleForm {
    pub role: String,
}

pub async fn set_role(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<RoleForm>,
) -> AppResult<Response> {
    if !ROLES.contains(&f.role.as_str()) {
        return Err(AppError::Domain(domain::DomainError::Invalid(
            "نقش نامعتبر است.".into(),
        )));
    }
    users_admin::set_role(&s.db, id, &f.role).await?;
    Ok(Redirect::to(&format!("/admin/users/{id}")).into_response())
}

// ===================== تنظیم دستی امتیاز =====================

#[derive(Deserialize)]
pub struct PointsForm {
    pub amount: String,
    pub direction: String, // add | deduct
    pub note: Option<String>,
}

pub async fn adjust_points(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<PointsForm>,
) -> AppResult<Response> {
    let Some(user) = users_admin::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "کاربر".into(),
        )));
    };
    let amount = parse_digits(f.amount.trim())
        .map_err(|m| AppError::Domain(domain::DomainError::Invalid(m)))?;
    let delta = if f.direction == "deduct" {
        -amount
    } else {
        amount
    };
    let note = f.note.as_deref().map(str::trim).filter(|s| !s.is_empty());
    db::loyalty::add_transaction(&s.db, user.id, delta, "manual", None, note).await?;
    Ok(Redirect::to(&format!("/admin/users/{id}")).into_response())
}

// ===================== تنظیم دستی کیف پول =====================

#[derive(Deserialize)]
pub struct WalletForm {
    pub amount_toman: String,
    pub direction: String, // add | deduct
    pub note: Option<String>,
}

pub async fn adjust_wallet(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Form(f): Form<WalletForm>,
) -> AppResult<Response> {
    let Some(user) = users_admin::find(&s.db, id).await? else {
        return Err(AppError::Domain(domain::DomainError::NotFound(
            "کاربر".into(),
        )));
    };
    let toman = parse_digits(f.amount_toman.trim())
        .map_err(|m| AppError::Domain(domain::DomainError::Invalid(m)))?;
    let delta = if f.direction == "deduct" {
        -(toman * 10)
    } else {
        toman * 10
    };
    let note = f.note.as_deref().map(str::trim).filter(|s| !s.is_empty());
    db::wallet::add_transaction(&s.db, user.id, delta, "admin", None, note)
        .await
        .map_err(|e| match e {
            sqlx::Error::Protocol(m) => AppError::Domain(domain::DomainError::Invalid(m)),
            other => AppError::Db(other),
        })?;
    Ok(Redirect::to(&format!("/admin/users/{id}")).into_response())
}

fn render<T: Template>(t: T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

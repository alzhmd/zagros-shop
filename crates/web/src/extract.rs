//! استخراج‌کننده‌ها: کاربر فعلی و کلید سبد از کوکی امضاشده.

use crate::auth::SESSION_COOKIE;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::SignedCookieJar;
use db::accounts::User;
use uuid::Uuid;

/// کاربر فعلی (اختیاری) — اگر کوکی نشست معتبر باشد.
pub struct CurrentUser(pub Option<User>);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::from_headers(&parts.headers, state.cookie_key.clone());
        let raw = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());
        let user = match raw {
            Some(val) => {
                // فرم جدید: توکن نشست (۶۴ هگز). فرم قدیمی: UUID مستقیم (سازگاری رو به عقب).
                let uid = if let Ok(uuid) = val.parse::<Uuid>() {
                    Some(uuid)
                } else {
                    db::sessions::user_for_token(&state.db, &val)
                        .await
                        .ok()
                        .flatten()
                };
                match uid {
                    Some(uid) => db::accounts::find_by_id(&state.db, uid)
                        .await
                        .ok()
                        .flatten(),
                    None => None,
                }
            }
            None => None,
        };
        Ok(CurrentUser(user))
    }
}

/// کاربری که باید نقش admin داشته باشد (وگرنه ریدایرکت).
/// فیلد User برای استفاده‌ی آینده نگه داشته شده (هندلرها فعلاً فقط به‌عنوان نگهبان استفاده می‌کنند).
#[allow(dead_code)]
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = axum::response::Redirect;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state)
            .await
            .unwrap_or(CurrentUser(None));
        match user {
            Some(u) if u.role == "admin" => Ok(AdminUser(u)),
            Some(_) => Err(axum::response::Redirect::to("/")),
            None => Err(axum::response::Redirect::to("/account/login")),
        }
    }
}

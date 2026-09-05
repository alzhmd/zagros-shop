use crate::{error::AppResult, state::AppState, web::render};
use askama::Template;
use axum::{extract::State, response::Html};

#[derive(Template)]
#[template(path = "pages/about.html")]
struct AboutTemplate {
    site_name: String,
}

pub async fn about(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(AboutTemplate {
        site_name: s.site_name(),
    })
}

#[derive(Template)]
#[template(path = "pages/contact.html")]
struct ContactTemplate {
    site_name: String,
    contact_phone: String,
    contact_address: String,
}

pub async fn contact(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(ContactTemplate {
        site_name: s.site_name(),
        contact_phone: s.settings.contact_phone(),
        contact_address: s.settings.contact_address(),
    })
}

#[derive(Template)]
#[template(path = "pages/faq.html")]
struct FaqTemplate {
    site_name: String,
}

pub async fn faq(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(FaqTemplate {
        site_name: s.site_name(),
    })
}

#[derive(Template)]
#[template(path = "pages/terms.html")]
struct TermsTemplate {
    site_name: String,
}

pub async fn terms(State(s): State<AppState>) -> AppResult<Html<String>> {
    render(TermsTemplate {
        site_name: s.site_name(),
    })
}

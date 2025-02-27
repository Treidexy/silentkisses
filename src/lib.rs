pub mod auth;
pub mod rooms;
pub mod res;
pub mod session;

use std::ops::Deref;

use axum::{extract::FromRef, http::StatusCode, response::{Html, IntoResponse, Response}};
use oauth2::reqwest;
use serde_json::Value;
use sqlx::SqlitePool;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub clients: auth::Clients,
}

pub trait GetField {
    fn get_str_field(&self, field: &str) -> AppResult<String>;
    fn get_obj_field(&self, field: &str) -> AppResult<&Value>;
}

impl GetField for serde_json::Value {
    fn get_str_field(&self, field: &str) -> AppResult<String> {
        Ok(
            self.get(field)
            .ok_or(format!("expected {field} in {self}"))?
            .as_str()
            .ok_or(format!("expected {field} in {self} to be string"))?
            .to_owned()
        )
    }
    
    fn get_obj_field(&self, field: &str) -> AppResult<&Value> {
        self.get(field)
        .ok_or(format!("expected {field} in {self}").into())
    }
}


pub type AppResult<T> = Result<T, AppError>;
#[derive(Debug)]
pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}\n\n{}", self.0, self.0.backtrace()),
        )
            .into_response()
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        Self(anyhow::Error::msg(err))
    }
}

impl From<&str> for AppError {
    fn from(err: &str) -> Self {
        Self(anyhow::Error::msg(err.to_owned()))
    }
}

macro_rules! apperr_impl {
    ($E:ty) => {
        impl From<$E> for AppError {
            fn from(err: $E) -> Self {
                Self(anyhow::Error::from(err))
            }
        }
    };
}

apperr_impl!(serde_json::Error);
apperr_impl!(sqlx::Error);
apperr_impl!(tower_sessions::session::Error);
apperr_impl!(axum::Error);
apperr_impl!(reqwest::Error);

// bc rust macros fucking suck
// apperr_impl!(oauth2::RequestTokenError<E: core::error::Error + Send + Sync + 'static, R: oauth2::ErrorResponse + Send + Sync + 'static>);

impl<E: core::error::Error + Send + Sync + 'static, R: oauth2::ErrorResponse + Send + Sync + 'static> From<oauth2::RequestTokenError<E, R>> for AppError {
    fn from(err: oauth2::RequestTokenError<E, R>) -> Self {
        Self(anyhow::Error::from(err))
    }
}

pub struct Markdown<T>(pub T);

impl<T> IntoResponse for Markdown<T>
where
    T: Deref<Target = str>
{
    fn into_response(self) -> axum::response::Response {
        use pulldown_cmark::{Event, Parser, Options};

        let parser = Parser::new_ext(&*self.0, Options::ENABLE_MATH)
            .map(|event| match event {
            Event::InlineMath(name) => Event::InlineMath(name),
            _ => event,
        });
    
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
        Html(html_output).into_response()
    }
}
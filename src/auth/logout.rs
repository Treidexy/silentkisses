use axum::{debug_handler, extract::Query, response::Redirect};
use serde::Deserialize;
use tower_sessions::Session;

use crate::AppResult;

#[derive(Deserialize)]
pub struct LogoutQuery {
    pub return_url: Option<String>,
}

#[debug_handler]
pub async fn logout(
    Query(LogoutQuery { return_url }): Query<LogoutQuery>,
    session: Session
) -> AppResult<Redirect> {
    session.clear().await;
    Ok(Redirect::to(return_url.unwrap_or("/".to_string()).as_str()))
}
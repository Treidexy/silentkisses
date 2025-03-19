mod page;
mod new;

use axum::{routing::get, Router};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{uuid}", get(page::profile))
}
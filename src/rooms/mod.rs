mod room;
mod msg;
mod new;
mod ws;

use axum::{debug_handler, extract::{Path, State}, response::{IntoResponse, Response}, routing::get, Json, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{auth, res, session::USER_ID, AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/new", get(new::new_room_page).post(new::new_room))
        .route("/{uuid}", get(room::room))
        .route("/{uuid}/ws", get(ws::room_ws))
}
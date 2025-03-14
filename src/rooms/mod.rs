mod room;
mod msg;
mod new;
mod ws;

use axum::{routing::get, Router};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/new", get(new::new_room_page).post(new::new_room))
        .route("/{uuid}", get(room::room))
        .route("/{uuid}/ws", get(ws::room_ws))
}
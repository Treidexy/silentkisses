use axum::{debug_handler, extract::State, response::{Html, IntoResponse, Redirect, Response}, Form};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{include_res, session::USER_ID, AppResult};

#[derive(Debug, Deserialize)]
pub(crate) struct NewRoomQuery {
    name: String,
    is_public: bool,
}

#[debug_handler]
pub(crate) async fn new_room_page(
    State(db_pool): State<SqlitePool>,
    session: Session,
) -> AppResult<Response> {
    if session.get::<String>(USER_ID).await?.is_none() {
        return Ok(Redirect::to("/login?return_url=/r/new").into_response());
    }

    Ok(Html(
        include_res!(str, "pages/new_room.html")
    ).into_response())
}

#[debug_handler]
pub(crate) async fn new_room(
    State(db_pool): State<SqlitePool>,
    session: Session,

    Form(NewRoomQuery { name, is_public }): Form<NewRoomQuery>,
) -> AppResult<Response> {
    let uuid = Uuid::now_v7();
    sqlx::query("INSERT INTO rooms (uuid,name,is_public) values (?,?,?)")
        .bind(uuid.to_string())
        .bind(&name)
        .bind(is_public)
        .execute(&db_pool)
        .await?;

    Ok(Redirect::to(
        &format!("/r/{uuid}")
    ).into_response())
}
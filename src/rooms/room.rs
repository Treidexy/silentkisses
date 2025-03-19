use axum::{debug_handler, extract::{Path, State}, response::{Html, IntoResponse, Response}};
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{include_res, res, session::USER_ID, AppResult};

use super::msg;

#[debug_handler]
pub(crate) async fn room(
    State(db_pool): State<SqlitePool>,
    session: Session,
    Path(room_id): Path<Uuid>,
) -> AppResult<Response> {
    let sorry = res::sorry("room");

    let Some((name, is_public)): Option<(String, bool)> =
        sqlx::query_as("SELECT name,is_public FROM rooms WHERE uuid=?")
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
    else {
        return sorry;
    };

    if !is_public {
        let Some(client_user_id) = session.get::<String>(USER_ID).await? else {
            return sorry;
        };

        if sqlx::query_as::<_, ()>("SELECT 1 FROM profiles WHERE uuid=? AND room_id=?")
            .bind(client_user_id)
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
            .is_none() {
            return sorry;
        }
    }

    let msgs: Vec<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id,profile_id,reply_to_id,content FROM messages WHERE room_id=?")
            .bind(room_id.to_string())
            .fetch_all(&db_pool)
            .await?;

    let mut messages = String::new();
    for (id, profile_id, reply_to_id, content) in msgs {
        messages += &msg::msg_to_html(
            Uuid::parse_str(&id)?,
            room_id, 
            Uuid::parse_str(&profile_id)?,
            match reply_to_id {
                Some(x) => Some(Uuid::parse_str(&x)?),
                None => None,
            },
            content, 
            &db_pool
        ).await?;
    }

    let body = include_res!(str, "pages/rooms/room.html")
        .replace("{room_id}", &room_id.to_string())
        .replace("{room_name}", &name)
        .replace("{messages}", &messages);

    return Ok(Html(body).into_response());
}
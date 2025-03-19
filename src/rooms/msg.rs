use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{include_res, AppResult};

#[derive(Deserialize)]
pub(crate) struct SendMessageQuery {
    reply_to_id: Option<Uuid>,
    content: String,
}

pub(crate) async fn send_msg(
    db_pool: &SqlitePool,
    tx: &broadcast::Sender<String>,
    
    profile_id: Uuid,
    room_id: Uuid,

    SendMessageQuery { reply_to_id, content }: SendMessageQuery,
) -> AppResult<Response> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO messages (id,room_id,profile_id,reply_to_id,content) values (?,?,?,?,?)")
        .bind(id.to_string())
        .bind(room_id.to_string())
        .bind(profile_id.to_string())
        .bind(reply_to_id.as_ref().map(Uuid::to_string))
        .bind(&content)
        .execute(db_pool)
        .await?;

    let _ = tx.send(
        msg_to_html(id, room_id, profile_id, reply_to_id, content, &db_pool).await?
    );

    Ok(().into_response())
}

pub(crate) async fn msg_to_html(
    id: Uuid,
    room_id: Uuid,
    profile_id: Uuid,
    reply_to_id: Option<Uuid>,
    content: String,

    db_pool: &SqlitePool,
) -> AppResult<String> {
    let (handle, alias): (String, String) =
        sqlx::query_as("SELECT handle,alias FROM profiles WHERE uuid=?")
            .bind(&profile_id.to_string())
            .fetch_optional(db_pool)
            .await?
            .unwrap_or(("?".to_owned(), "Anonymous".to_owned()));

    let mut content_html = String::new();
    pulldown_cmark::html::push_html(&mut content_html, pulldown_cmark::Parser::new(&content));

    let mut message = include_res!(str, "pages/rooms/message.html")
        .replace("{alias}", &alias)
        .replace("{handle}", &handle)
        .replace("{id}", &id.to_string())
        .replace("{profile_id}", &profile_id.to_string())
        .replace("{content}", &content_html);

    if let Some(reply_to_id) = reply_to_id {
        let (reply_to,): (String,) =
            sqlx::query_as("SELECT content FROM messages WHERE id=? AND room_id=?")
                .bind(reply_to_id.to_string())
                .bind(room_id.to_string())
                .fetch_one(db_pool)
                .await?;

        message = message
            .replace("{reply_to_id}", &reply_to_id.to_string())
            .replace("{reply_to}", &reply_to);
    } else {
        message = message
            .replace("{reply_to_id}", "")
            .replace("{reply_to}", "");
    }

    Ok(message)
}
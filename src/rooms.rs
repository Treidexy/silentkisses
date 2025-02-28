use axum::{
    debug_handler,
    extract::{Path, State, WebSocketUpgrade},
    response::{Html, IntoResponse, Response}, Json,
};
use pulldown_cmark::Parser;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{auth, include_res, session::USER_ID, AppResult};

#[derive(Deserialize)]
pub struct MessageQuery {
    reply_to_id: Option<Uuid>,
    content: String,
}

#[debug_handler]
pub async fn private_room() -> impl IntoResponse {
    Html(include_res!(str, "pages/private_room.html"))
}

#[debug_handler]
pub async fn room(
    Path(room_id): Path<Uuid>,
    State(db_pool): State<SqlitePool>,
    session: Session,
) -> AppResult<Response> {
    let Some((name, is_public)): Option<(String, bool)> =
        sqlx::query_as("SELECT name,is_public FROM rooms WHERE uuid=?")
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
    else {
        return Err(format!("r/{room_id} don't exist lil bro 2"))?;
    };

    if !is_public {
        let Some(client_user_id) = session.get::<String>(USER_ID).await? else {
            return Err(format!("sign in to access r/{room_id}"))?;
        };

        if sqlx::query_as::<_, ()>("SELECT 1 FROM profiles WHERE uuid=? AND room_id=?")
            .bind(client_user_id)
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
            .is_none() {
                return Err(format!("your account doesn't have access to r/{room_id}"))?;
        }
    }

    let msgs: Vec<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id,profile_id,reply_to_id,content FROM messages WHERE room_id=?")
            .bind(room_id.to_string())
            .fetch_all(&db_pool)
            .await?;

    let mut messages = String::new();
    for (id, profile_id, reply_to_id, content) in msgs {
        messages += &msg_to_html(
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

    let body = include_res!(str, "pages/room.html")
        .replace("{room_id}", &room_id.to_string())
        .replace("{room_name}", &name)
        .replace("{messages}", &messages);

    return Ok(Html(body).into_response());
}

#[debug_handler]
pub async fn room_ws(
    ws: WebSocketUpgrade,
    State(tx): State<broadcast::Sender<String>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut stream| async move {
        let mut rx = tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            if stream.send(msg.into()).await.is_err() {
                break;
            }
        }
    })
}

#[debug_handler(state = crate::AppState)]
pub async fn send_msg(
    Path(room_id): Path<Uuid>,
    State(db_pool): State<SqlitePool>,
    State(tx): State<broadcast::Sender<String>>,
    session: Session,

    Json(MessageQuery { reply_to_id, content }): Json<MessageQuery>,
) -> AppResult<impl IntoResponse> {
    let Some(user_id) = session.get::<String>(USER_ID).await? else {
        return Err("must sign in")?;
    };

    let Some((is_public,)): Option<(bool,)> =
        sqlx::query_as("SELECT is_public FROM rooms WHERE uuid=?")
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
    else {
        return Err(format!("r/{room_id} don't exist lil bro 2"))?;
    };

    let profile_id: Option<(String,)> = sqlx::query_as("SELECT uuid FROM profiles WHERE user_id=? AND room_id=?")
        .bind(&user_id)
        .bind(room_id.to_string())
        .fetch_optional(&db_pool)
        .await?;
    
    let profile_id = if let Some((profile_id,)) = profile_id {
        Uuid::parse_str(&profile_id)?
    } else if is_public {
        auth::create_profile(&db_pool, &user_id, &room_id.to_string()).await?.0
    } else {
        return Err(format!("you are not in r/{room_id}"))?;
    };

    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO messages (id,room_id,profile_id,reply_to_id,content) values (?,?,?,?,?)")
        .bind(id.to_string())
        .bind(room_id.to_string())
        .bind(profile_id.to_string())
        .bind(reply_to_id.as_ref().map(Uuid::to_string))
        .bind(&content)
        .execute(&db_pool)
        .await?;

    let _ = tx.send(
        msg_to_html(id, room_id, profile_id, reply_to_id, content, &db_pool).await?
    );

    Ok(())
}

pub async fn msg_to_html(
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
    pulldown_cmark::html::push_html(&mut content_html, Parser::new(&content));

    let mut message = include_res!(str, "pages/message.html")
        .replace("{alias}", &alias)
        .replace("{handle}", &handle)
        .replace("{id}", &id.to_string())
        .replace("{profile_id}", &profile_id.to_string())
        .replace("{content}", &content_html);

    if let Some(reply_to_id) = reply_to_id {
        let (mut reply_to,): (String,) =
            sqlx::query_as("SELECT content FROM messages WHERE id=? AND room_id=?")
                .bind(reply_to_id.to_string())
                .bind(room_id.to_string())
                .fetch_one(db_pool)
                .await?;
        let other = reply_to.split_off(20);
        if !other.is_empty() {
            reply_to += "...";
        }

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
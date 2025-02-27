use std::{fmt, ops::Deref};

use axum::{
    debug_handler,
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};
use pulldown_cmark::Parser;
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{include_res, session::USER_ID, AppResult};

#[debug_handler]
pub async fn private_room(
    session: Session,
    State(db_pool): State<SqlitePool>,
) -> AppResult<impl IntoResponse> {
    if let Some(user_id) = session.get::<String>("user_id").await? {
        let (alias,): (String,) =
            sqlx::query_as("SELECT alias FROM profiles WHERE user_id=? AND room_id=0")
                .bind(user_id)
                .fetch_one(&db_pool)
                .await?;

        return Ok(Html(format!(
            r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Silent Hugs</title>
        </head>
        <body>
            <a href='/'>go home</a>
            <br>
            <h1>Welcome {}!</h1>
        </body>
        </html>"#,
            alias
        )));
    }

    Ok(Html(
        "Welcome to the private room, <a href='/login'>Log In</a><br><a href='/'>go home</a>"
            .to_string(),
    ))
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
        return Err(format!("{room_id} don't exist lil bro 2"))?;
    };

    if !is_public {
        let Some(user_id) = session.get::<String>(USER_ID).await? else {
            return Err(format!("sign in to access the private room, {room_id}"))?;
        };

        if sqlx::query_as::<_, ()>("SELECT 1 FROM profiles WHERE uuid=? AND room_id=?")
            .bind(user_id)
            .bind(room_id.to_string())
            .fetch_optional(&db_pool)
            .await?
            .is_none() {
                return Err(format!("your account doesn't have access to the private room, {room_id}"))?;
        }
    }

    let msgs: Vec<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id,profile_id,reply_to_id,content FROM messages WHERE room_id=?")
            .bind(room_id.to_string())
            .fetch_all(&db_pool)
            .await?;

    let mut messages = String::new();
    for (id, profile_id, reply_to_id, content) in msgs {
        let (user_id, handle, alias): (String, String, String) =
            sqlx::query_as("SELECT uuid,handle,alias FROM profiles WHERE uuid=?")
                .bind(profile_id)
                .fetch_optional(&db_pool)
                .await?
                .unwrap_or((String::new(), "?".to_owned(), "Anonymous".to_owned()));

        let mut content_html = String::new();
        pulldown_cmark::html::push_html(&mut content_html, Parser::new(&content));

        let mut message = include_res!(str, "pages/message.html")
            .replace("{alias}", &alias)
            .replace("{handle}", &handle)
            .replace("{id}", &id)
            .replace("{user_id}", &user_id)
            .replace("{content}", &content_html);

        if let Some(reply_to_id) = reply_to_id {
            let (mut reply_to,): (String,) =
                sqlx::query_as("SELECT content FROM messages WHERE id=? AND room_id=?")
                    .bind(reply_to_id.to_string())
                    .bind(room_id.to_string())
                    .fetch_one(&db_pool)
                    .await?;
            let other = reply_to.split_off(20);
            if !other.is_empty() {
                reply_to += "...";
            }

            message = message
                .replace("{reply_to_id}", &reply_to_id)
                .replace("{reply_to}", &reply_to)
        } else {
            message = message
                .replace("{reply_to_id}", "")
                .replace("{reply_to}", "");
        }

        messages += &message;
    }

    let body = include_res!(str, "pages/room.html")
        .replace("{room_name}", &name)
        .replace("{messages}", &messages);

    return Ok(Html(body).into_response());
}

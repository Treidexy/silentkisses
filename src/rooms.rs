use axum::{debug_handler, extract::{Path, State}, response::{Html, IntoResponse}};
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::AppResult;

#[debug_handler]
pub async fn private_room(session: Session, State(db_pool): State<SqlitePool>) -> AppResult<impl IntoResponse> {
    if let Some(user_id) = session.get::<String>("user_id").await? {
        let (alias,): (String,) = sqlx::query_as("SELECT alias FROM profiles WHERE user_id=? AND room_id=0").bind(user_id).fetch_one(&db_pool).await?;

        return Ok(Html(format!(r#"<!DOCTYPE html>
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
        </html>"#, alias)));
    }

    Ok(Html("Welcome to the private room, <a href='/login'>Log In</a><br><a href='/'>go home</a>".to_string()))
}

#[debug_handler]
pub async fn room(Path(uuid): Path<Uuid>, State(db_pool): State<SqlitePool>) -> String {
    let result: Option<(i64,String)> = sqlx::query_as("SELECT rowid,name FROM rooms WHERE uuid=?")
        .bind(uuid.to_string())
        .fetch_optional(&db_pool)
        .await.unwrap();
    if let Some((room_id,name)) = result {
        return format!("Welcome to {name}#{room_id} ({uuid})");
    }

    format!("{uuid} don't exist lil bro 2")
}
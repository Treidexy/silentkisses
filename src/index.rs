use axum::{debug_handler, extract::State, response::{Html, IntoResponse, Redirect, Response}};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::{include_res, AppResult};

#[debug_handler]
pub async fn index(
    State(db_pool): State<SqlitePool>,
    session: Session
) -> AppResult<Response> {
    let Some(user_id) = session.get::<String>("user_id").await? else {
        return Ok(
            Redirect::to("/login")
                .into_response()
        );
    };

    let mut room_items = String::new();
    let room_ids = sqlx::query_as::<_, (String,)>("SELECT room_id,alias FROM profiles WHERE user_id=?")
        .bind(&user_id)
        .fetch_all(&db_pool)
        .await?;
    for (room_id,) in room_ids {
        if room_id == "0" {
            continue;
        }

        let (name,): (String,) = sqlx::query_as("SELECT name FROM rooms WHERE uuid=?")
            .bind(&room_id)
            .fetch_one(&db_pool)
            .await?;

        room_items += &include_res!(str, "pages/room_item.html")
            .replace("{id}", &room_id)
            .replace("{name}", &name);
    }

    Ok(
        Html(
            include_res!(str, "/pages/index.html")
                .replace("{room_items}", &room_items)
        ).into_response()
    )
}

use axum::{debug_handler, extract::{Path, State}, response::{Html, IntoResponse, Response}, routing::get, Router};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{include_res, session::USER_ID, AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{uuid}", get(profile))
}

#[debug_handler]
async fn profile(
    Path(profile_id): Path<Uuid>,
    State(db_pool): State<SqlitePool>,
    session: Session,
) -> AppResult<Response> {
    let sorry = Err((
        StatusCode::FORBIDDEN,
        Html(
            include_res!(str, "pages/sorry.html")
            .replace("{service}", "profile")
        )
    ).into_response().into());

    let Some(user_id) = session.get::<String>(USER_ID).await? else {
        return sorry;
    };

    let (room_id, handle, alias): (String, String, String) = sqlx::query_as("SELECT room_id,handle,alias FROM profiles WHERE uuid=?")
        .bind(profile_id.to_string())
        .fetch_one(&db_pool)
        .await?;

    if sqlx::query("SELECT 1 FROM profiles WHERE user_id=? AND room_id=?")
        .bind(&user_id)
        .bind(&room_id)
        .fetch_optional(&db_pool)
        .await?.is_none() {
        return sorry;
    }

    let (room_name,): (String,) = sqlx::query_as("SELECT name FROM rooms WHERE uuid=?")
        .bind(&room_id)
        .fetch_one(&db_pool)
        .await?;

    Ok(Html(
        include_res!(str, "pages/profile.html")
        .replace("{alias}", &alias)
        .replace("{handle}", &handle)
        .replace("{room_id}", &room_id)
        .replace("{room_name}", &room_name)
    ).into_response())
}
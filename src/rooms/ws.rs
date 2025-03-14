use axum::{debug_handler, extract::{Path, State, WebSocketUpgrade}, response::IntoResponse};
use futures_util::{SinkExt, StreamExt};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{auth, res, rooms::msg, session::USER_ID, AppResult};

#[debug_handler(state = crate::AppState)]
pub async fn room_ws(
    Path(room_id): Path<Uuid>,
    State(db_pool): State<SqlitePool>,
    State(tx): State<broadcast::Sender<String>>,
    session: Session,

    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user_id = session.get::<String>(USER_ID).await.unwrap().unwrap();

    let (is_public,): (bool,) =
        sqlx::query_as("SELECT is_public FROM rooms WHERE uuid=?")
            .bind(room_id.to_string())
            .fetch_one(&db_pool)
            .await
            .unwrap();

    let profile_id: Option<(String,)> = sqlx::query_as("SELECT uuid FROM profiles WHERE user_id=? AND room_id=?")
        .bind(&user_id)
        .bind(room_id.to_string())
        .fetch_optional(&db_pool)
        .await
        .unwrap();

    let profile_id = if let Some((profile_id,)) = profile_id {
        Uuid::parse_str(&profile_id).unwrap()
    } else if is_public {
        auth::create_profile(&db_pool, &user_id, &room_id.to_string()).await.unwrap().0
    } else {
        panic!();
    };

    ws.on_upgrade(async move |stream| {
        let mut rx = tx.subscribe();
        let (mut sender, mut receiver) = stream.split();

        let mut broadcast_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if sender.send(msg.into()).await.is_err() {
                    break;
                }
            }
        });

        while let Some(Ok(msg)) = receiver.next().await {
            let Ok(msg) = serde_json::from_slice(&msg.into_data()) else {
                continue
            };

            let _ = msg::send_msg(&db_pool, &tx, profile_id, room_id, msg).await;
        }

        tokio::select! {
            _ = &mut broadcast_task => broadcast_task.abort(),
        };
    })
}
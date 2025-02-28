use std::str::FromStr;

use silentkisses::{auth, include_res, rooms, AppResult, AppState, Markdown};
use axum::{
    debug_handler, response::{Html, IntoResponse}, routing::get, Router
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::broadcast;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(1)));

    let db_pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect(dotenv::var("DATABASE_URL").unwrap().as_str())
        .await.unwrap();

    let clients = auth::Clients::from_json(serde_json::Value::from_str(include_str!("../client_secret.json")).unwrap()).unwrap();
    let app_state = AppState {
        db_pool,
        clients,
        tx: broadcast::channel(69).0
    };

    let app = Router::new()
        .route("/", get(hello))
        .route("/test", get(test))


        .route("/login", get(login))
        .route("/login/{provider}", get(auth::login))
        .route("/lockin/{provider}", get(auth::lockin))
        .route("/logout", get(auth::logout))

        .route("/r/0", get(rooms::private_room))
        .route("/r/{uuid}", get(rooms::room).post(rooms::send_msg))
        .route("/r/{uuid}/ws", get(rooms::room_ws))

        .with_state(app_state)
        .layer(session_layer);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn test() -> impl IntoResponse {
    Markdown(include_res!(str, "/pages/hello.md"))
}

#[debug_handler]
async fn hello(
    session: Session
) -> AppResult<impl IntoResponse> {
    let p = if session.get::<String>("user_id").await?.is_some() {
        include_res!(str, "/pages/index_logout.html")
    } else {
        include_res!(str, "/pages/index_login.html")
    };

    Ok(Html(p))
}

#[debug_handler]
async fn login() -> impl IntoResponse {
    Html(include_res!(str, "/pages/login.html"))
}


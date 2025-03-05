use std::str::FromStr;

use silentkisses::{auth, include_res, profiles, rooms, AppResult, AppState, Markdown};
use axum::{
    debug_handler, response::{Html, IntoResponse, Redirect, Response}, routing::get, Router
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::broadcast;
use tower_sessions::{cookie::SameSite, Expiry, MemoryStore, Session, SessionManagerLayer};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(5)));

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

        .merge(auth::router())
        .nest("/r", rooms::router())
        .nest("/p", profiles::router())

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
) -> AppResult<Response> {
    if session.get::<String>("user_id").await?.is_some() {
        Ok(
            Html(include_res!(str, "/pages/index.html"))
            .into_response()
        )
    } else {
        Ok(
            Redirect::to("/login")
                .into_response()
        )
    }
}

use std::str::FromStr;

use silentkisses::{auth, AppResult, AppState};
use axum::{
    debug_handler, extract::{Path, State}, response::{Html, IntoResponse}, routing::get, Router
};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::seconds(10)));

    let db_pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect(dotenv::var("DATABASE_URL").unwrap().as_str())
        .await.unwrap();

    let clients = auth::Clients::from_json(serde_json::Value::from_str(include_str!("../client_secret.json")).unwrap()).unwrap();
    let app_state = AppState {
        db_pool,
        clients,
    };

    let app = Router::new()
        .route("/", get(hello))
        .route("/res/background.jpg", get(res_background))

        .route("/login", get(login))
        .route("/login/{provider}", get(auth::login))
        .route("/lockin/{provider}", get(auth::lockin))
        .route("/logout", get(auth::logout))
        .route("/r/0", get(private_room))
        .route("/r/{uuid}", get(room))
        .with_state(app_state)
        .layer(session_layer);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn hello(
    session: Session
) -> AppResult<impl IntoResponse> {
    let p = if session.get::<String>("user_id").await?.is_some() {
        include_str!("pages/index_logout.html")
    } else {
        include_str!("pages/index_login.html")
    };

    Ok(Html(p))
}

#[debug_handler]
async fn res_background() -> impl IntoResponse {
    include_bytes!("../res/background.jpg")
}

#[debug_handler]
async fn login() -> impl IntoResponse {
    Html(include_str!("pages/login.html"))
}

#[debug_handler]
async fn private_room(session: Session, State(db_pool): State<SqlitePool>) -> AppResult<impl IntoResponse> {
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
async fn room(Path(uuid): Path<Uuid>, State(db_pool): State<SqlitePool>) -> String {
    let result: Option<(i64,String)> = sqlx::query_as("SELECT rowid,name FROM rooms WHERE uuid=?")
        .bind(uuid.to_string())
        .fetch_optional(&db_pool)
        .await.unwrap();
    if let Some((room_id,name)) = result {
        return format!("Welcome to {name}#{room_id} ({uuid})");
    }

    format!("{uuid} don't exist lil bro 2")
}
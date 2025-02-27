use std::{ops::Deref, str::FromStr};

use silentkisses::{auth, rooms, res, include_res, AppResult, AppState};
use axum::{
    debug_handler, response::{Html, IntoResponse}, routing::get, Router
};
use sqlx::sqlite::SqlitePoolOptions;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};

struct Markdown<T>(T);

impl<T> IntoResponse for Markdown<T>
where
    T: Deref<Target = str>
{
    fn into_response(self) -> axum::response::Response {
        use pulldown_cmark::{Event, Parser, Options};

        let parser = Parser::new_ext(&*self.0, Options::ENABLE_MATH)
            .map(|event| match event {
            Event::InlineMath(name) => Event::InlineMath(name),
            _ => event,
        });
    
        let mut html_output = format!("<style>{}</style>", include_res!(str, "/style.css"));
        pulldown_cmark::html::push_html(&mut html_output, parser);
        Html(html_output).into_response()
    }
}

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
    };

    let app = Router::new()
        .route("/", get(hello))
        .route("/test", get(test))
        .route("/res/background.jpg", get(res::background))

        .route("/login", get(login))
        .route("/login/{provider}", get(auth::login))
        .route("/lockin/{provider}", get(auth::lockin))
        .route("/logout", get(auth::logout))

        .route("/r/0", get(rooms::private_room))
        .route("/r/{uuid}", get(rooms::room))
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


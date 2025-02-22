use std::str::FromStr;

use anyhow::anyhow;

use axum::{
    debug_handler, extract::{FromRef, Path, Query, State}, http::StatusCode, response::{Html, IntoResponse, Redirect, Response}, routing::get, Extension, Router
};
use oauth2::{basic::{BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse, BasicTokenResponse, BasicTokenType}, reqwest, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, EndpointNotSet, ExtraTokenFields, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, StandardRevocableToken, StandardTokenResponse, TokenResponse, TokenType, TokenUrl};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use uuid::Uuid;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db_pool: SqlitePool,
}

#[tokio::main]
async fn main() {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::seconds(10)));

    let db_pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect(dotenv::var("DATABASE_URL").unwrap().as_str())
        .await.unwrap();
    let app_state = AppState {db_pool};

    let app = Router::new()
        .route("/", get(hello))
        .route("/login", get(login))
        .route("/yippee", get(yippee))
        .route("/r/{uuid}", get(room))
        // .layer(CorsLayer::permissive())
        .with_state(app_state)
        .layer(session_layer);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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

type AppResult<T> = Result<T, AppError>;
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[debug_handler]
async fn hello(
    State(db_pool): State<SqlitePool>,
    session: Session
) -> AppResult<impl IntoResponse> {
    if let Some(user_id) = session.get::<String>("user_id").await? {
        let (alias,): (String,) = sqlx::query_as("SELECT alias FROM profiles WHERE user_id=? AND room_id=0").bind(user_id).fetch_one(&db_pool).await?;

        return Ok(Html(format!(r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Silent Hugs</title>
        </head>
        <body>
            <h1>Welcome {}!</h1>
        </body>
        </html>"#, alias)));
    }

    Ok(Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Silent Hugs</title>
</head>
<body>
    <a href='/login'><h1>Log in</h1></a>
</body>
</html>"#)))
}

#[derive(Deserialize)]
struct LoginQuery {
    return_url: Option<String>,
}

#[debug_handler]
async fn login(
    Query(LoginQuery { return_url }): Query<LoginQuery>,
    session: Session
) -> AppResult<Redirect> {
    let return_url = return_url.unwrap_or("/".to_string());

    let client = get_client()?;
    
    let (pkce_code_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    session.insert("csrf_state", csrf_state.secret()).await?;
    session.insert("pkce_verifier", pkce_verifier.secret()).await?;
    session.insert("return_url", return_url).await?;

    Ok(Redirect::to(authorize_url.as_str()))
}

#[derive(Deserialize)]
struct OAuth2ReturnQuery {
    state: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenIdExtraFields {
    id_token: String,
}

impl ExtraTokenFields for OpenIdExtraFields {}

#[debug_handler]
async fn yippee(
    Query(OAuth2ReturnQuery { state, code }): Query<OAuth2ReturnQuery>,
    State(db_pool): State<SqlitePool>,
    session: Session,
) -> AppResult<impl IntoResponse> {
    let state = CsrfToken::new(state.unwrap_or("OAuth: without state".to_string()));
    let code = AuthorizationCode::new(code.unwrap_or("OAuth: without code".to_string()));

    let stored_state: String = session.get("csrf_state").await?.unwrap();
    if state.secret().as_str() != stored_state.as_str() {
        return Err(anyhow!("csrf tokens don't match").into());
    }

    let pkce_verifier: String = session.get("pkce_verifier").await?.unwrap();
    
    let client = get_client()?;
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let token_result = client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
        .request_async(&http_client)
        .await?;

    let access_token = token_result.access_token().secret();
    let id_token = &token_result.extra_fields().id_token;
    let body = reqwest::get("https://oauth2.googleapis.com/tokeninfo?id_token=".to_owned() + id_token).await?.text().await?;
    println!("b = {body}");

    
    let url = "https://www.googleapis.com/oauth2/v2/userinfo?oauth_token=".to_owned() + access_token;
    let body = reqwest::get(url).await?.text().await?;
    let mut body: serde_json::Value = serde_json::from_str(body.as_str())?;
    
    println!("body = {body}");
    return Ok(Redirect::to("/"));
    let user_id = body["id"].take().as_str().unwrap().to_string();
    let return_url: String = session.get("return_url").await?.unwrap();
    
    session.insert("user_id", user_id.clone()).await?;
    
    let query: Result<(String,String), _> = sqlx::query_as(r#"SELECT user_id,alias FROM profiles WHERE user_id=? AND room_id=0"#)
        .bind(user_id.as_str())
        .fetch_one(&db_pool)
        .await;
    match query {
        Ok((user_id, alias)) => {
            println!("welcome {alias}#{user_id}");
        }
        Err(sqlx::Error::RowNotFound) => {
            let name = body["name"].take().as_str().unwrap().to_string();
            println!("adding {user_id}");
            sqlx::query("INSERT INTO profiles (user_id,room_id,alias) VALUES (?,0,?)")
                .bind(user_id)
                .bind(name)
                .execute(&db_pool)
                .await?;
        }
        Err(e) => {
            return Err(AppError(anyhow::Error::from(e)));
        }
    }

    Ok(Redirect::to(return_url.as_str()))
}

fn get_client() -> anyhow::Result<Client<oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>, StandardTokenResponse<OpenIdExtraFields, BasicTokenType>, oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>, StandardRevocableToken, oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>, oauth2::EndpointSet, EndpointNotSet, EndpointNotSet, oauth2::EndpointSet, oauth2::EndpointSet>>  {
    let mut client_secret: serde_json::Value = serde_json::from_str(include_str!("../client_secret.json"))?;
    let web = client_secret["web"].take();

    fn ez(web: &serde_json::Value, key: &str) -> String {
        String::from_str(web[key].as_str().unwrap()).unwrap()
    }

    let id = ClientId::new(ez(&web, "client_id"));
    let secret = ClientSecret::new(ez(&web, "client_secret"));

    let auth_url = AuthUrl::new(ez(&web, "auth_uri"))?;
    let token_url = TokenUrl::new(ez(&web, "token_uri"))?;
    let revoke_url = RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())?;

    let redirect_url = RedirectUrl::new("http://localhost:8080/yippee".to_string())?;
    let client = Client::<
        BasicErrorResponse,
        StandardTokenResponse<OpenIdExtraFields, BasicTokenType>,
        BasicTokenIntrospectionResponse,
        StandardRevocableToken,
        BasicRevocationErrorResponse,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
    >::new(id)
        .set_client_secret(secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url)
        .set_revocation_url(revoke_url);

    Ok(client)
}
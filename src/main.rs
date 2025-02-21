use std::{collections::HashMap, str::FromStr};

use axum::{
    debug_handler, extract::Query, http::StatusCode, response::{IntoResponse, Redirect, Response}, routing::get, Extension, Router
};
use oauth2::{basic::BasicClient, reqwest, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenResponse, TokenUrl};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};

#[derive(Clone, Debug)]
struct UserData {
    id: String,
}

#[tokio::main]
async fn main() {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(30)));

    let app = Router::new()
        .route("/", get(hello))
        .route("/login", get(login))
        .route("/yippee", get(yippee))
        // .layer(CorsLayer::permissive())
        .layer(session_layer)
        .layer(Extension(Option::<UserData>::None));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
async fn hello() -> String {
    "Hello".into()
}

#[debug_handler]
async fn login(
    Query(mut params): Query<HashMap<String, String>>,
    session: Session,
) -> AppResult<Redirect> {
    let return_url = params
        .remove("return_url")
        .unwrap_or_else(|| "/".to_string());

    let client = get_client()?;
    
    let (pkce_code_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    session.insert("csrf_state", csrf_state.secret()).await?;
    session.insert("pkce_verifier", pkce_verifier.secret()).await?;

    Ok(Redirect::to(authorize_url.as_str()))
}

#[debug_handler]
async fn yippee(
    Query(mut params): Query<HashMap<String, String>>,
    session: Session,
) -> AppResult<impl IntoResponse> {
    let state = CsrfToken::new(params.remove("state").unwrap_or("OAuth: without state".to_string()));
    let code = AuthorizationCode::new(params.remove("code").unwrap_or("OAuth: without code".to_string()));

    let stored_state: String = session.get("csrf_state").await?.unwrap();
    if state.secret().as_str() != stored_state.as_str() {
        Err::<(), &str>("csrf tokens don't match").unwrap();
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
    let url = "https://www.googleapis.com/oauth2/v2/userinfo?oauth_token=".to_owned() + access_token;
    let body = reqwest::get(url).await?.text().await?;
    let mut body: serde_json::Value = serde_json::from_str(body.as_str())?;
    
    let id = body["id"].take().as_str().unwrap().to_string();

    // meh, I'll think abt it later
    // let session_token_p1 = Uuid::new_v4().to_string();
    // let session_token_p2 = Uuid::new_v4().to_string();
    // let session_token = [session_token_p1.as_str(), "_", session_token_p2.as_str()].concat();
    // let headers = axum::response::AppendHeaders([(
    //     axum::http::header::SET_COOKIE,
    //     format!("session_token={session_token}; path=/; httponly; secure; samesite=strict"),
    // )]);

    Ok(Redirect::to("/"))
}

fn get_client() -> anyhow::Result<Client<oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>, oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardRevocableToken, oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>, oauth2::EndpointSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointSet, oauth2::EndpointSet>>  {
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
    let client = BasicClient::new(id)
        .set_client_secret(secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url)
        .set_revocation_url(revoke_url);

    Ok(client)
}
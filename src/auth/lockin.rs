use axum::{debug_handler, extract::{Path, Query, State}, response::{IntoResponse, Redirect}};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeVerifier, TokenResponse};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::{AppResult, AppState, GetField};

use super::{clients::ClientProvider, create_user, Clients};

#[derive(Deserialize)]
pub struct LockinQuery {
    pub state: Option<String>,
    pub code: Option<String>,
}

#[derive(Serialize)]
struct FirebaseRequest {
    post_body: String,
    request_uri: String,
    return_idp_credential: bool,
    return_secure_token: bool,
}

#[debug_handler(state = AppState)]
pub async fn lockin(
    Path(provider): Path<ClientProvider>,
    Query(LockinQuery { state, code }): Query<LockinQuery>,
    State(db_pool): State<SqlitePool>,
    State(clients): State<Clients>,
    session: Session,
) -> AppResult<impl IntoResponse> {
    let state = CsrfToken::new(state.ok_or("OAuth: without state")?);
    let code = AuthorizationCode::new(code.ok_or("OAuth: without code")?);

    let Some(stored_state) = session.get::<String>("csrf_state").await? else {
        return Err("no csrf_state")?;
    };

    if state.secret().as_str() != stored_state.as_str() {
        return Err("csrf tokens don't match")?;
    }

    let Some(pkce_verifier) = session.get::<String>("pkce_verifier").await? else {
        return Err("no pkce_verifier")?;
    };
    
    let client = clients.get_client(provider)?;
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let token_result = client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
        .request_async(&http_client)
        .await?;

    let access_token = token_result.access_token().secret();
    let body: serde_json::Value = http_client.post(clients.firebase_idpurl)
        .json(&FirebaseRequest {
            post_body: format!("access_token={access_token}&providerId={}", provider.id()),
            request_uri: "http://localhost/".to_owned(),
            return_idp_credential: true,
            return_secure_token: true,
        })
        .send()
        .await?
        .json()
        .await?;

    
    let user_id = body.get_str_field("localId")?;
    session.insert("user_id", user_id.clone()).await?;

    let mut return_url = session.get("return_url").await?;
    
    let query: Result<(String,String,String), _> = sqlx::query_as(r#"SELECT user_id,handle,alias FROM profiles WHERE user_id=? AND room_id=0"#)
        .bind(user_id.as_str())
        .fetch_one(&db_pool)
        .await;
    match query {
        Ok((user_id, handle, alias)) => {
            println!("welcome @{handle}#{user_id}, {alias}");
        }
        Err(sqlx::Error::RowNotFound) => {
            create_user(db_pool, user_id).await?;

            if let None = return_url {
                return_url = Some("/r/0".to_string());
            }
        }
        Err(e) => {
            return Err(e)?;
        }
    }

    let return_url: String = return_url.unwrap_or("/".to_string());
    Ok(Redirect::to(return_url.as_str()))
}
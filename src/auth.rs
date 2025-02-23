use std::str::FromStr;

use axum::{debug_handler, extract::{Query, State}, response::{ErrorResponse, IntoResponse, Redirect}};
use oauth2::{basic::BasicClient, reqwest, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenResponse, TokenUrl};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;

use anyhow::anyhow;
use uuid::Uuid;

use crate::appresult::{AppError, AppResult};

#[derive(Deserialize)]
pub struct ReturnUrlQuery {
    pub return_url: Option<String>,
}

#[derive(Deserialize)]
pub struct OAuth2ReturnQuery {
    pub state: Option<String>,
    pub code: Option<String>,
}

async fn create_user(db_pool: SqlitePool, user_id: String, alias: String) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let uuid = Uuid::now_v7();
    let handle = "user".to_owned() + &uuid.simple().to_string();

    println!("adding @{handle}#{user_id}, {alias}");
    sqlx::query("insert into profiles (uuid,user_id,room_id,handle,alias) VALUES (?,?,0,?,?)")
        .bind(uuid.to_string())
        .bind(user_id)
        .bind(handle)
        .bind(alias)
        .execute(&db_pool)
        .await
}

#[debug_handler]
pub async fn login(
    Query(ReturnUrlQuery { return_url }): Query<ReturnUrlQuery>,
    session: Session
) -> AppResult<Redirect> {
    let client = get_client()?;
    
    let (pkce_code_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    session.insert("csrf_state", csrf_state.secret()).await?;
    session.insert("pkce_verifier", pkce_verifier.secret()).await?;
    if let Some(return_url) = return_url {
        session.insert("return_url", return_url).await?;
    }

    Ok(Redirect::to(authorize_url.as_str()))
}

#[debug_handler]
pub async fn yippee(
    Query(OAuth2ReturnQuery { state, code }): Query<OAuth2ReturnQuery>,
    State(db_pool): State<SqlitePool>,
    session: Session,
) -> AppResult<impl IntoResponse> {
    let state = CsrfToken::new(state.ok_or(anyhow!("OAuth: without state"))?);
    let code = AuthorizationCode::new(code.ok_or(anyhow!("OAuth: without code"))?);

    let stored_state: String = match session.get("csrf_state").await? {
        Some(x) => x,
        None => {
            return Err(anyhow!("no csrf_state"))?;
        }
    };
    if state.secret().as_str() != stored_state.as_str() {
        return Err(anyhow!("csrf tokens don't match").into());
    }

    let pkce_verifier: String = match session.get("pkce_verifier").await? {
        Some(x) => x,
        None => {
            return Err(anyhow!("no pkce_verifier"))?; 
        }
    };
    
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
    
    let user_id = body["id"].take().as_str().ok_or(anyhow!("Couldn't read id from response"))?.to_string();
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
            let alias = body["name"].take().as_str().unwrap_or("Nameless User").to_string();
            
            create_user(db_pool, user_id, alias).await?;

            if return_url.is_none() {
                return_url = Some("/r/0".to_string());
            }
        }
        Err(e) => {
            return Err(AppError(anyhow::Error::from(e)));
        }
    }

    let return_url: String = return_url.unwrap_or("/".to_string());

    Ok(Redirect::to(return_url.as_str()))
}

#[debug_handler]
pub async fn logout(
    Query(ReturnUrlQuery { return_url }): Query<ReturnUrlQuery>,
    session: Session
) -> AppResult<Redirect> {
    session.clear().await;
    Ok(Redirect::to(return_url.unwrap_or("/".to_string()).as_str()))
}

fn get_client() -> anyhow::Result<Client<oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>, oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardRevocableToken, oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>, oauth2::EndpointSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointSet, oauth2::EndpointSet>> {
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
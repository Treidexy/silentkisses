use axum::{debug_handler, extract::{Path, Query, State}, response::{IntoResponse, Redirect, Response}};
use oauth2::{CsrfToken, PkceCodeChallenge, Scope};
use serde::Deserialize;
use tower_sessions::Session;

use crate::AppResult;

use super::{clients::ClientProvider, Clients};

#[derive(Deserialize)]
pub struct LoginQuery {
    pub return_url: Option<String>,
}

#[debug_handler]
pub async fn login(
    Path(provider): Path<ClientProvider>,
    Query(LoginQuery { return_url }): Query<LoginQuery>,
    State(clients): State<Clients>,
    session: Session,
) -> AppResult<Response> {
    let client = clients.get_client(provider)?;
    
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

    Ok(Redirect::to(authorize_url.as_str()).into_response())
}
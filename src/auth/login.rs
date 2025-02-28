use axum::{debug_handler, extract::{Path, Query, State}, response::{Html, IntoResponse, Redirect, Response}};
use oauth2::{CsrfToken, PkceCodeChallenge, Scope};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{include_res, session::{CSRF_STATE, PKCE_VERIFIER, RETURN_URL}, AppResult};

use super::{clients::ClientProvider, Clients};

#[derive(Deserialize)]
pub(crate) struct LoginQuery {
    pub(crate) return_url: Option<String>,
}

#[debug_handler]
pub(crate) async fn login_page() -> impl IntoResponse {
    Html(include_res!(str, "/pages/login.html"))
}


#[debug_handler]
pub(crate) async fn login(
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

    session.insert(CSRF_STATE, csrf_state.secret()).await?;
    session.insert(PKCE_VERIFIER, pkce_verifier.secret()).await?;
    if let Some(return_url) = return_url {
        session.insert(RETURN_URL, return_url).await?;
    }

    dbg!(session.get_value(CSRF_STATE).await?);

    Ok(Redirect::to(authorize_url.as_str()).into_response())
}
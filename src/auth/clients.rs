use std::fmt;

use oauth2::{basic::BasicClient, AuthUrl, Client, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;
use serde_json::Value;

use crate::{AppResult, GetField};

type HappyClient = Client<oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>, oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, oauth2::StandardRevocableToken, oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>, oauth2::EndpointSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointSet>;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ClientProvider {
    Google,
    Github,
}

impl ClientProvider {
    pub fn id(&self) -> &str {
        use ClientProvider::*;
        match self {
            Google => "google.com",
            Github => "github.com",
        }
    }
}

impl fmt::Display for ClientProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Clients {
    pub(crate) firebase_idpurl: String,
    google_client: Option<HappyClient>,
    github_client: Option<HappyClient>,
}

impl Clients {
    pub fn from_json(json: Value) -> AppResult<Clients> {
        let firebase_idpurl = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signInWithIdp?key={}",
            json.get_obj_field("firebase")?.get_str_field("apikey")?
        );
        let google_client = 'a: {
            let json = json.get("google");
            let Some(json) = json else {
                break 'a None;
            };
            let client_id = ClientId::new(json.get_str_field("client_id")?);
            let client_secret = ClientSecret::new(json.get_str_field("client_secret")?);

            let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string()).unwrap();
            let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap();
            let redirect_url = RedirectUrl::new("http://localhost:8080/lockin/google".to_owned()).unwrap();

            Some(
                BasicClient::new(client_id)
                .set_client_secret(client_secret)
                .set_auth_uri(auth_url)
                .set_token_uri(token_url)
                .set_redirect_uri(redirect_url)
            )
        };
        let github_client = 'a: {
            let json = json.get("github");
            let Some(json) = json else {
                break 'a None;
            };
            let client_id = ClientId::new(json.get_str_field("client_id")?);
            let client_secret = ClientSecret::new(json.get_str_field("client_secret")?);

            let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
            let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap();
            let redirect_url = RedirectUrl::new("http://localhost:8080/lockin/github".to_owned()).unwrap();

            Some(
                BasicClient::new(client_id)
                .set_client_secret(client_secret)
                .set_auth_uri(auth_url)
                .set_token_uri(token_url)
                .set_redirect_uri(redirect_url)
            )
        };

        Ok(
            Clients {
                firebase_idpurl,
                google_client,
                github_client,
            }
        )
    }

    pub fn get_client(&self, provider: ClientProvider) -> AppResult<HappyClient> {
        use ClientProvider::*;
        match provider {
            Google => self.google_client.clone(),
            Github => self.github_client.clone(),
        }.ok_or(format!("OAuth provider {provider} keys not supplied").into())
    }
}
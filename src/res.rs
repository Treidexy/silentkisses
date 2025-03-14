use axum::response::{Html, IntoResponse, Response};
use reqwest::StatusCode;

use crate::AppResult;

#[macro_export]
macro_rules! include_res {
    (bytes, $p:expr) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/", $p))
    };
    (str, $p:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/", $p))
    };
    (string, $p:expr) => {
        include_res!(str, $p).to_owned()
    };
}

pub fn sorry(service: &str) -> AppResult<Response> {
    Err((
        StatusCode::FORBIDDEN,
        Html(
            include_res!(str, "pages/sorry.html")
            .replace("{service}", "room")
        )
    ).into_response().into())
}
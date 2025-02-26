use axum::{debug_handler, response::IntoResponse};

#[macro_export]
macro_rules! include_res {
    (bytes, $p:expr) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res", $p))
    };
    (str, $p:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res", $p))
    };
}

#[debug_handler]
pub async fn background() -> impl IntoResponse {
    include_res!(bytes, "/background.jpg")
}
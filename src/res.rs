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
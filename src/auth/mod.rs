use axum::{routing::get, Router};
use rand::seq::IndexedRandom;
use sqlx::SqlitePool;

use uuid::Uuid;

mod clients;
mod login;
mod lockin;
mod logout;

pub use clients::Clients;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login::login_page))
        .route("/login/{provider}", get(login::login))
        .route("/lockin/{provider}", get(lockin::lockin))
        .route("/logout", get(logout::logout))
}

pub(crate) async fn create_profile(db_pool: &SqlitePool, user_id: &str, room_id: &str) -> Result<(Uuid, sqlx::sqlite::SqliteQueryResult), sqlx::Error> {
    let uuid = Uuid::now_v7();
    let handle = format!("user{}", uuid.simple());
    let adjectives = [
        "Quick", "Lazy", "Mysterious", "Jolly", "Brave", "Silent", "Witty", "Fierce",
        "Clever", "Gentle", "Wild", "Calm", "Bold", "Shy", "Proud", "Happy", "Sad",
        "Eager", "Fancy", "Rusty", "Golden", "Silver", "Bright", "Dark", "Lucky",
    ];
    
    let nouns = [
        "Fox", "Bear", "Eagle", "Wolf", "Dragon", "Tiger", "Lion", "Owl", "Rabbit",
        "Falcon", "Hawk", "Shark", "Panda", "Kitten", "Puppy", "Phoenix", "Griffin",
        "Unicorn", "Turtle", "Dolphin", "Whale", "Elephant", "Giraffe", "Zebra",
    ];
    
    let alias = format!("{} {}", adjectives.choose(&mut rand::rng()).unwrap(), nouns.choose(&mut rand::rng()).unwrap());
    
    println!("adding @{handle}#{user_id}, {alias} to {room_id}");
    sqlx::query("insert into profiles (uuid,user_id,room_id,handle,alias) VALUES (?,?,?,?,?)")
        .bind(uuid.to_string())
        .bind(user_id)
        .bind(room_id)
        .bind(handle)
        .bind(alias)
        .execute(db_pool)
        .await
        .map(|query| (uuid, query))
}
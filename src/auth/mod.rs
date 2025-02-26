use rand::seq::IndexedRandom;
use sqlx::SqlitePool;

use uuid::Uuid;

mod clients;
mod login;
mod lockin;
mod logout;

pub use clients::Clients;
pub use login::login;
pub use lockin::lockin;
pub use logout::logout;

pub(crate) async fn create_user(db_pool: SqlitePool, user_id: String) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let uuid = Uuid::now_v7();
    let handle = "user".to_owned() + &uuid.simple().to_string();
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
    
    println!("adding @{handle}#{user_id}, {alias}");
    sqlx::query("insert into profiles (uuid,user_id,room_id,handle,alias) VALUES (?,?,0,?,?)")
        .bind(uuid.to_string())
        .bind(user_id)
        .bind(handle)
        .bind(alias)
        .execute(&db_pool)
        .await
}
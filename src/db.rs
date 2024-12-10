use std::env;

use dotenv::dotenv;
use mongodb::{Client, Collection};
use reg_state::RegState;

pub mod reg_state;



pub struct DB{
    pub reg_states: Collection<RegState>
}

impl DB{
    pub async fn init() -> Self{
        dotenv().ok();
        let mongo_uri = env::var("DB_URL").expect("No var named DB_URL found!");
        let client = Client::with_uri_str(mongo_uri).await.expect("Failed connecting to the database");
        println!("Connected to database!");
        let database = client.database("polling-app");
        let reg_state_collection = database.collection("reg_states");

        DB{
            reg_states: reg_state_collection
        }
    }
}
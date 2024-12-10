use std::env;

use dotenv::dotenv;
use mongodb::Client;
use reg_state_repo::RegStateRepo;
use users_repo::UserRepo;

pub mod reg_state_repo;
pub mod users_repo;

pub struct DB{
    pub reg_states: RegStateRepo,
    pub users: UserRepo
}

impl DB{
    pub async fn init() -> Self{
        dotenv().ok();
        let mongo_uri = env::var("DB_URL").expect("No var named DB_URL found!");
        let client = Client::with_uri_str(mongo_uri).await.expect("Failed connecting to the database");
        println!("Connected to database!");
        let database = client.database("polling-app");
        let reg_state_collection = RegStateRepo::init(&database).await;
        let users_collection = UserRepo::init(&database).await;
        DB{
            reg_states: reg_state_collection,
            users: users_collection
        }
    }
}
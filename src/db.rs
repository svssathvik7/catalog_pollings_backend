use std::sync::{Arc, Mutex};

use auth_state_repo::AuthStateRepo;
use log::debug;
use mongodb::Client;
use options_repo::OptionRepo;
use polls_repo::PollRepo;
use reg_state_repo::RegStateRepo;
use users_repo::UserRepo;

use crate::config::app_config::AppConfig;

pub mod auth_state_repo;
pub mod options_repo;
pub mod polls_repo;
pub mod reg_state_repo;
pub mod users_repo;

pub struct DB {
    pub client: Client,
    pub reg_states: RegStateRepo,
    pub users: UserRepo,
    pub auth_states: AuthStateRepo,
    pub options: OptionRepo,
    pub polls: PollRepo,
}

impl DB {
    pub async fn init(app_config: Arc<AppConfig>) -> Arc<Mutex<Self>> {
        let mongo_uri = &app_config.db_url;
        let client = Client::with_uri_str(mongo_uri)
            .await
            .expect("Failed connecting to the database");
        debug!("Connected to database!");
        let database = client.database("polling-app");
        let reg_state_collection = RegStateRepo::init(&database).await;
        let auth_state_collection = AuthStateRepo::init(&database).await;
        let users_collection = UserRepo::init(&database).await;
        let options_collection = OptionRepo::init(&database).await;
        let polls_collection = PollRepo::init(&database).await;
        let db_instance = DB {
            client,
            reg_states: reg_state_collection,
            users: users_collection,
            auth_states: auth_state_collection,
            options: options_collection,
            polls: polls_collection,
        };
        Arc::new(Mutex::new(db_instance))
    }
}

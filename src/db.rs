use std::sync::{Arc, Mutex};

use crate::config::app_config::AppConfig;
use auth_state_repo::AuthStateRepo;
use log::error;
use mongodb::Client;
use options_repo::OptionRepo;
use polls_repo::PollRepo;
use reg_state_repo::RegStateRepo;
use tokio::try_join;
use users_repo::UserRepo;
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
    pub async fn init(app_config: Arc<AppConfig>) -> Result<Arc<Mutex<Self>>, ()> {
        let mongo_uri = &app_config.db_url;
        let client = Client::with_uri_str(mongo_uri)
            .await
            .expect("Failed connecting to the database");
        println!("Connected to database!");
        let database = client.database("polling-app");
        let (reg_states, auth_states, users, options, polls) = try_join!(
            RegStateRepo::init(&database),
            AuthStateRepo::init(&database),
            UserRepo::init(&database),
            OptionRepo::init(&database),
            PollRepo::init(&database)
        )
        .map_err(|e| error!("Error initializing collection: {}", e))?;
        let db_instance = DB {
            client,
            reg_states,
            users,
            auth_states,
            options,
            polls,
        };
        Ok(Arc::new(Mutex::new(db_instance)))
    }
}

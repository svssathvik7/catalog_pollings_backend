use std::error::Error;

use mongodb::{bson::doc, Collection, Database};
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize)]
pub struct User {
    pub username: String,
    pub uuid: String,
    pub sk: serde_json::Value,
}

pub struct UserRepo {
    collection: Collection<User>,
}

impl UserRepo {
    pub async fn init(db: &Database) -> Self {
        let users_collection = db.collection("users");
        Self {
            collection: users_collection,
        }
    }

    pub async fn search_by_username(&self,username: &str) -> Result<Option<User>,Box<dyn Error>>{
        let filter = doc! {username: username};
        let result = self.collection.find_one(filter).await?;
        Ok(result)
    }
}

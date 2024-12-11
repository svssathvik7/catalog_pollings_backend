use std::error::Error;

use mongodb::{bson::doc, results::InsertOneResult, Collection, Database};
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
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await?;
        Ok(result)
    }

    pub async fn insert(&self,new_user: User) -> Result<InsertOneResult,mongodb::error::Error>{
        let result = self.collection.insert_one(new_user).await;
        result
    }   

    // O(1) instead of find_by_username's O(n) for checking
    pub async fn is_exists(&self,username:&str) -> Result<bool, mongodb::error::Error>{
        let filter= doc! {"username": username};
        let exists = match self.collection.count_documents(filter).await {
            Ok(count) => Ok(count>0),
            Err(e) => {
                eprint!("Error counting documents with username {:?}",e);
                Ok(false)
            }
        };
        exists
    }
}

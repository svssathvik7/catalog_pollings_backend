use log::error;
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::InsertOneResult,
    Collection, Database, IndexModel,
};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
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
        let index = IndexModel::builder()
            .keys(doc! {"username": 1})
            .options(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .name(Some("unique_username".to_string()))
                    .build(),
            )
            .build();

        if let Err(e) = users_collection.create_index(index).await {
            error!("Failed to create index on `username`: {:?}", e);
        }
        Self {
            collection: users_collection,
        }
    }

    pub async fn search_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn Error>> {
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await?;
        Ok(result)
    }

    pub async fn get_user_id(
        &self,
        username: &str,
    ) -> Result<Option<ObjectId>, mongodb::error::Error> {
        let filter = doc! {"username": username};
        let result = match self.collection.find_one(filter).await? {
            Some(user) => user.id,
            None => {
                error!("No user found!");
                return Ok(None);
            }
        };
        Ok(result)
    }

    pub async fn insert(&self, new_user: User) -> Result<InsertOneResult, mongodb::error::Error> {
        let result = self.collection.insert_one(new_user).await;
        result
    }

    // O(1) instead of find_by_username's O(n) for checking
    pub async fn is_exists(&self, username: &str) -> Result<bool, mongodb::error::Error> {
        let filter = doc! {"username": username};
        let exists = match self.collection.count_documents(filter).await {
            Ok(count) => Ok(count > 0),
            Err(e) => {
                error!("Error counting documents with username {:?}", e);
                Ok(false)
            }
        };
        exists
    }

    pub async fn query_by_filter(
        &self,
        filter: mongodb::bson::Document,
    ) -> Result<Option<User>, mongodb::error::Error> {
        let result = self.collection.find_one(filter).await;

        result
    }
}

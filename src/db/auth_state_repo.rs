use std::error::Error;

use anyhow::Result;
use log::{debug, error};
use mongodb::{
    bson::doc,
    results::{DeleteResult, InsertOneResult},
    Collection, Database, IndexModel,
};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthState {
    pub username: String,
    pub auth_state: serde_json::Value,
}

pub struct AuthStateRepo {
    pub collection: Collection<AuthState>,
}

impl AuthStateRepo {
    pub async fn init(db: &Database) -> Result<Self, Box<dyn Error>> {
        let auth_state_collection: Collection<AuthState> = db.collection("auth_states");

        let index = IndexModel::builder()
            .keys(doc! {"username": 1})
            .options(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .name(Some("unique_username".to_string()))
                    .build(),
            )
            .build();

        if let Err(e) = auth_state_collection.create_index(index).await {
            error!("Failed to create index on `username`: {:?}", e);
        }

        Ok(Self {
            collection: auth_state_collection,
        })
    }

    pub async fn insert(&self, auth_state_entry: AuthState) -> Result<InsertOneResult> {
        let username = &auth_state_entry.username;
        let _auth_state_alread_exists = match self.is_exists(username).await {
            Ok(data) => {
                if data {
                    // cleanup any existing auth states
                    self.delete_by_username(username).await?;
                } else {
                    ()
                }
            }
            Err(e) => {
                error!("Error deleting a duplicate auth state record {:?}", e);
            }
        };
        let result = self
            .collection
            .insert_one(auth_state_entry)
            .await
            .map_err(|e| anyhow::Error::new(e));
        result
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<AuthState>> {
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await.map_err(|e| {
            error!("Error finding auth state by username {}", e);
            anyhow::Error::new(e)
        });
        debug!("found {:?}", result);
        result
    }

    pub async fn is_exists(&self, username: &str) -> Result<bool> {
        let filter = doc! {"username": username};
        Ok(self.collection.find_one(filter).await?.is_some())
    }

    pub async fn delete_by_username(&self, username: &str) -> Result<DeleteResult> {
        let filter = doc! {"username": username};
        let result = self.collection.delete_one(filter).await.map_err(|e| {
            error!("Error deleteing auth state by username {}", e);
            anyhow::Error::new(e)
        });
        result
    }
}

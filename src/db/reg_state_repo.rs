use anyhow::Result;
use log::error;
use mongodb::{
    bson::doc,
    results::{DeleteResult, InsertOneResult},
    Collection, Database, IndexModel,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct RegState {
    pub username: String,
    pub uuid: String,
    pub reg_state: serde_json::Value,
}
pub struct RegStateRepo {
    collection: Collection<RegState>,
}

impl RegStateRepo {
    pub async fn init(db: &Database) -> Result<Self, Box<dyn Error>> {
        let reg_state_collection: Collection<RegState> = db.collection("reg_states");
        let index = IndexModel::builder()
            .keys(doc! {"username": 1})
            .options(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .name(Some("unique_username".to_string()))
                    .build(),
            )
            .build();

        if let Err(e) = reg_state_collection.create_index(index).await {
            error!("Failed to create index on `username`: {:?}", e);
        }
        Ok(Self {
            collection: reg_state_collection,
        })
    }

    pub async fn insert(&self, reg_state_entry: RegState) -> Result<InsertOneResult> {
        let username = &reg_state_entry.username;
        let _reg_state_alread_exists = match self.is_exists(username).await {
            Ok(data) => {
                if data {
                    self.delete_by_username(username).await.unwrap();
                } else {
                    ()
                }
            }
            Err(e) => {
                error!("Error deleting a duplicate record {:?}", e);
            }
        };
        let result = self
            .collection
            .insert_one(reg_state_entry)
            .await
            .map_err(|e| {
                error!("Error inserting reg state {}", e);
                anyhow::Error::new(e)
            });
        result
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<RegState>> {
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await.map_err(|e| {
            error!("Error finding reg state by username {}", e);
            anyhow::Error::new(e)
        });
        result
    }

    pub async fn is_exists(&self, username: &str) -> Result<bool> {
        let filter = doc! {"username": username};

        Ok(self.collection.find_one(filter).await?.is_some())
    }

    pub async fn delete_by_username(&self, username: &str) -> Result<DeleteResult> {
        let filter = doc! {"username": username};
        let result = self.collection.delete_many(filter).await.map_err(|e| {
            error!("Error deleting reg state by username {}", e);
            anyhow::Error::new(e)
        });
        result
    }
}

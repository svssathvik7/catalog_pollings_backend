use mongodb::{bson::doc, results::{DeleteResult, InsertOneResult}, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct AuthState{
    pub username: String,
    pub auth_state: serde_json::Value
}

pub struct AuthStateRepo{
    pub collection: Collection<AuthState>
}

impl AuthStateRepo{
    pub async fn init(db: &Database) -> Self{
        let auth_state_collection: Collection<AuthState> = db.collection("auth_states");

        let index = IndexModel::builder().keys(doc! {"username": 1}).options(
            mongodb::options::IndexOptions::builder().unique(true).name(Some("unique_username".to_string())).build()
        ).build();

        if let Err(e) = auth_state_collection.create_index(index).await {
            eprintln!("Failed to create index on `username`: {:?}", e);
        }

        Self{
            collection: auth_state_collection
        }
    }

    pub async fn insert(&self,auth_state_entry: AuthState) -> Result<InsertOneResult,mongodb::error::Error>{
        let username = &auth_state_entry.username;
        let _auth_state_alread_exists = match self.is_exists(username).await{
            Ok(data) => {
                if data{
                    self.delete_by_username(username).await?;
                }
                else{
                    ()
                }
            },
            Err(e) => {
                eprint!("Error deleting a duplicate record {:?}",e);
            }
        };
        let result = self.collection.insert_one(auth_state_entry).await;
        result
    }

    pub async fn find_by_username(&self,username: &str) -> Result<Option<AuthState>,mongodb::error::Error>{
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await;
        print!("found {:?}",result);
        result
    }

    pub async fn is_exists(&self,username: &str) -> Result<bool,mongodb::error::Error>{
        let filter = doc! {"username": username};
        Ok(self.collection.find_one(filter).await?.is_some())
    }

    pub async fn delete_by_username(&self,username: &str) -> Result<DeleteResult,mongodb::error::Error>{
        let filter = doc! {"username": username};
        let result = self.collection.delete_one(filter).await;
        result
    }
}
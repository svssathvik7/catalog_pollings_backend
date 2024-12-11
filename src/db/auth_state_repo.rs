use mongodb::{bson::doc, results::InsertOneResult, Collection, Database};
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
        Self{
            collection: auth_state_collection
        }
    }

    pub async fn insert(&self,auth_state_entry: AuthState) -> Result<InsertOneResult,mongodb::error::Error>{
        let result = self.collection.insert_one(auth_state_entry).await;
        result
    }

    pub async fn find_by_username(&self,username: &str) -> Result<Option<AuthState>,mongodb::error::Error>{
        let filter = doc! {"username": username};
        let result = self.collection.find_one(filter).await;
        print!("found {:?}",result);
        result
    }
}
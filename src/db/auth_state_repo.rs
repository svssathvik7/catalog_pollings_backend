use mongodb::{bson::doc, results::{DeleteResult, InsertOneResult}, Collection, Database};
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

    pub async fn is_exists(&self,username: &str) -> Result<bool,mongodb::error::Error>{
        let filter = doc! {"username": username};
        let result = match self.collection.count_documents(filter).await {
            Ok(data) => Ok(data>0),
            Err(e) => {
                eprint!("Error counting documents with username in auth state repo {:?}",e);
                Ok(false)
            }
        };
        result
    }

    pub async fn delete_by_username(&self,username: &str) -> Result<DeleteResult,mongodb::error::Error>{
        let filter = doc! {"username": username};
        let result = self.collection.delete_one(filter).await;
        result
    }
}
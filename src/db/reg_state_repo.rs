use mongodb::{bson::doc, results::{DeleteResult, InsertOneResult}, Collection, Database};
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct RegState {
    pub username: String,
    pub uuid: String,
    pub reg_state: serde_json::Value,
}
pub struct RegStateRepo {
    collection: Collection<RegState>,
}


impl RegStateRepo {
    pub async fn init(db: &Database) -> Self {
        let reg_state_collection: Collection<RegState> = db.collection("reg_states");
        Self {
            collection: reg_state_collection,
        }
    }

    pub async fn insert(&self,reg_state_entry: RegState) -> Result<InsertOneResult,mongodb::error::Error>{
        let username = &reg_state_entry.username;
        let _reg_state_alread_exists = match self.is_exists(username).await{
            Ok(data) => {
                if data{
                    self.delete_by_username(username).await.unwrap();
                }
                else{
                    ()
                }
            },
            Err(e) => {
                eprint!("Error deleting a duplicate record {:?}",e);
            }
        };
        let result = self.collection.insert_one(reg_state_entry).await;
        result
    }

    pub async fn find_by_username(&self,username: &str) -> Result<Option<RegState>,mongodb::error::Error>{
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
        let result = self.collection.delete_many(filter).await;
        result
    }
}

use mongodb::{bson::oid::ObjectId, results::InsertOneResult, Collection, Database};
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize,Debug)]
pub struct Poll{
    pub title: String,
    pub owner_id: ObjectId,
    pub options: Vec<ObjectId>
}

pub struct PollRepo{
    pub collection: Collection<Poll>
}

impl PollRepo{
    pub fn init(db: &Database) -> Self{
        let polls_repo = db.collection("polls");
        Self{
            collection: polls_repo
        }
    }

    pub async fn insert(&self,new_poll: Poll) -> Result<InsertOneResult,mongodb::error::Error>{
        let result = self.collection.insert_one(new_poll).await;
        result
    }
}
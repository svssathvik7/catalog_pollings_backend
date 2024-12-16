use futures::TryStreamExt;
use mongodb::{bson::{doc, oid::ObjectId, Document}, results::InsertOneResult, Collection, Database};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Poll {
    pub title: String,
    pub owner_id: ObjectId,
    pub options: Vec<ObjectId>,
}

pub struct PollRepo {
    pub collection: Collection<Poll>,
}

impl PollRepo {
    pub async fn init(db: &Database) -> Self {
        let polls_repo = db.collection("polls");
        Self {
            collection: polls_repo,
        }
    }

    pub async fn insert(&self, new_poll: Poll) -> Result<InsertOneResult, mongodb::error::Error> {
        let result = self.collection.insert_one(new_poll).await;
        result
    }

    pub async fn get(&self,poll_id: ObjectId) -> Result<Option<Document>,mongodb::error::Error>{
        let pipeline = vec![
            doc! {"_id" : poll_id},
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "expanded_options"
                }
            },
            doc! {
                "$project": {
                    "title": 1,
                    "owner_id": 1,
                    "expanded_options": 1
                }
            }
        ];
        let mut cursor = self.collection.aggregate(pipeline).await?;
        
        // Use try_next() to get the first result
        let result = cursor.try_next().await?;

        Ok(result)
    }
}

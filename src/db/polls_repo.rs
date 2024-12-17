use futures::TryStreamExt;
use mongodb::{bson::{doc, oid::ObjectId, Document}, results::InsertOneResult, Collection, Database};
use serde::{Deserialize, Serialize};

use super::DB;

#[derive(Deserialize, Serialize, Debug)]
pub struct Poll {
    pub id: String,
    pub title: String,
    pub owner_id: ObjectId,
    pub options: Vec<ObjectId>,
    pub is_open: bool
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

    pub async fn get(&self,poll_id: &str) -> Result<Option<Document>,mongodb::error::Error>{
        println!("{:?}",poll_id);
        let pipeline = vec![
            doc! {
                "$match" : {
                    "id": poll_id
                }
            },
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "options"
                }
            },
            doc! {
                "$project": {
                    "title": 1,
                    "owner_id": 1,
                    "options": 1
                }
            }
        ];
        let mut cursor = self.collection.aggregate(pipeline).await?;
        
        // Use try_next() to get the first result
        let result = cursor.try_next().await?;

        Ok(result)
    }

    pub async fn close_poll(&self,poll_id: &str) -> Result<bool,mongodb::error::Error>{
        let filter = doc! {"id":poll_id};
        let result = match self.collection.update_one(filter, doc!{"status": false}).await {
            Ok(_document) => true,
            Err(e) => {return Err(e);}
        };
        Ok(result)
    }

    pub async fn reset_poll(&self,poll_id: &str,db: &DB) -> Result<bool,mongodb::error::Error>{
        let poll_match = match self.get(poll_id).await? {
            Some(poll) => poll,
            None => {
                return Ok(false);
            }
        };
        let options_ids: Vec<ObjectId> = poll_match.get_array("options").unwrap_or(&Vec::new()).iter().filter_map(|option| option.as_object_id().clone()).collect();

        for option_id in options_ids{
            let filter = doc! {"_id": option_id};
            db.options.delete(filter).await?;
        }

        let filter = doc! {"id": poll_id};
        let update = doc! {
            "$set": {
                "options": Vec::<ObjectId>::new(),
                "is_open": true
            }
        };

        let result = match self.collection.update_one(filter, update).await {
            Ok(_) => true,
            Err(e) => return Err(e),
        };

        Ok(result)
    }
}

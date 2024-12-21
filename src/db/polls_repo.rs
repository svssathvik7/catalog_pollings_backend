use futures::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    results::InsertOneResult,
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::DB;

#[derive(Deserialize, Serialize, Debug)]
pub struct Poll {
    pub id: String,
    pub title: String,
    pub owner_id: ObjectId,
    pub options: Vec<ObjectId>,
    pub is_open: bool,
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

    pub async fn get(&self, poll_id: &str) -> Result<Option<Document>, mongodb::error::Error> {
        println!("{:?}", poll_id);
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
            },
        ];
        let mut cursor = self.collection.aggregate(pipeline).await?;

        // Use try_next() to get the first result
        let result = cursor.try_next().await?;

        Ok(result)
    }

    pub async fn is_owner(&self, poll_id: &str, user_id: &str) -> bool {
        match self.get(poll_id).await {
            Ok(Some(poll)) => {
                if let Some(owner_id) = poll.get("owner_id") {
                    return owner_id.as_str() == Some(user_id);
                }
                false
            }
            Ok(None) => false,
            Err(_) => false,
        }
    }

    pub async fn add_vote(
        &self,
        poll_id: &str,
        user_id: ObjectId,
        option_id: ObjectId,
        db: &DB,
    ) -> Result<bool, mongodb::error::Error> {
        // 1. Fetch poll details
        let poll_doc = match self.get(poll_id).await? {
            Some(poll) => poll,
            None => return Ok(false), // Poll not found
        };

        // 2. Check poll status
        let is_open = poll_doc.get_bool("is_open").unwrap_or(false);
        if !is_open {
            return Ok(false); // Poll is closed
        }

        // 3. Validate if option belongs to this poll
        let poll_options: Vec<ObjectId> = poll_doc
            .get_array("options")
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|opt| opt.as_object_id().clone())
            .collect();

        if !poll_options.contains(&option_id) {
            return Ok(false); // Option not part of this poll
        }

        // 4. Check if user has already voted in this poll
        let option_collection = db.options.collection.clone();

        // Find the specific option document
        let option_filter = doc! {"_id": option_id};
        let option_doc = match option_collection.find_one(option_filter.clone()).await? {
            Some(doc) => doc,
            None => return Ok(false), // Option not found
        };

        // 5. Check if user has already voted
        let current_voters = option_doc.voters;

        if current_voters.contains(&user_id) {
            return Ok(false); // User has already voted
        }

        // 6. Prepare update operations
        // Atomically update both the option document
        let update_option = doc! {
            "$inc": { "votes_count": 1 },
            "$push": { "voters": user_id }
        };

        // Begin a multi-document transaction for consistency
        let mut session = db.client.start_session().await?;
        session.start_transaction().await?;

        let update_result = match option_collection
            .update_one(option_filter, update_option)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                session.abort_transaction().await?;
                return Err(e);
            }
        };

        // 7. Verify update
        if update_result.modified_count != 1 {
            session.abort_transaction().await?;
            return Ok(false);
        }

        // 8. Commit transaction
        session.commit_transaction().await?;

        Ok(true)
    }

    pub async fn close_poll(
        &self,
        poll_id: &str,
        user_id: ObjectId,
    ) -> Result<bool, mongodb::error::Error> {
        if !self.is_owner(poll_id, &user_id.to_string()).await {
            return Ok(false);
        }
        let filter = doc! {"id":poll_id};
        let result = match self
            .collection
            .update_one(filter, doc! {"status": false})
            .await
        {
            Ok(_document) => true,
            Err(e) => {
                return Err(e);
            }
        };
        Ok(result)
    }

    // get live polls i.e status: true with decreasing votes count way, and we get page number, polls per page params to ensure pagination 

    // get closed polls i.e status: false with decreasing votes count way, and we get page number, polls per page params to ensure pagination 

    pub async fn reset_poll(
        &self,
        poll_id: &str,
        db: &DB,
        user_id: ObjectId,
    ) -> Result<bool, mongodb::error::Error> {
        if !self.is_owner(poll_id, &user_id.to_string()).await {
            return Ok(false);
        }
        let poll_match = match self.get(poll_id).await? {
            Some(poll) => poll,
            None => {
                return Ok(false);
            }
        };
        let options_ids: Vec<ObjectId> = poll_match
            .get_array("options")
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|option| option.as_object_id().clone())
            .collect();

        for option_id in options_ids {
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

    pub async fn get_live_polls(
        &self, 
        page: u64, 
        per_page: u64
    ) -> Result<Vec<Document>, mongodb::error::Error> {
        // Validate pagination parameters
        let page = page.max(1);
        let per_page = per_page.clamp(1, 10);
        
        // Calculate skip for pagination
        let skip = (page - 1) * per_page;
    
        let pipeline = vec![
            // Match only open polls
            doc! {
                "$match": {
                    "is_open": true
                }
            },
            // First lookup to expand options
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "expanded_options"
                }
            },
            // Second lookup for poll owner details
            doc! {
                "$lookup": {
                    "from": "users",
                    "localField": "owner_id",
                    "foreignField": "_id",
                    "as": "owner"
                }
            },
            // Unwind owner array (since it will be single document)
            doc! {
                "$unwind": {
                    "path": "$owner",
                    "preserveNullAndEmptyArrays": true
                }
            },
            // For each option, expand its voters
            doc! {
                "$lookup": {
                    "from": "users",
                    "let": { "expanded_options": "$expanded_options" },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$expanded_options.voters"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": 1,
                                "username": 1,
                                "uuid": 1
                            }
                        }
                    ],
                    "as": "voter_details"
                }
            },
            // Map voters to their respective options
            doc! {
                "$addFields": {
                    "expanded_options": {
                        "$map": {
                            "input": "$expanded_options",
                            "as": "option",
                            "in": {
                                "$mergeObjects": [
                                    "$$option",
                                    {
                                        "voters": {
                                            "$map": {
                                                "input": {
                                                    "$filter": {
                                                        "input": "$voter_details",
                                                        "as": "voter",
                                                        "cond": {
                                                            "$in": ["$$voter._id", "$$option.voters"]
                                                        }
                                                    }
                                                },
                                                "as": "voter",
                                                "in": {
                                                    "_id": "$$voter._id",
                                                    "username": "$$voter.username",
                                                    "uuid": "$$voter.uuid"
                                                }
                                            }
                                        }
                                    }
                                ]
                            }
                        }
                    }
                }
            },
            // Calculate total votes
            doc! {
                "$addFields": {
                    "total_votes": {
                        "$sum": "$expanded_options.votes_count"
                    }
                }
            },
            // Sort by total votes in descending order
            doc! {
                "$sort": {
                    "total_votes": -1
                }
            },
            // Pagination
            doc! {
                "$skip": skip as i64
            },
            doc! {
                "$limit": per_page as i64
            },
            // Final projection
            doc! {
                "$project": {
                    "_id": 1,
                    "id": 1,
                    "title": 1,
                    "total_votes": 1,
                    "is_open": 1,
                    "owner_username": "$owner.username",
                    "options": {
                        "$map": {
                            "input": "$expanded_options",
                            "as": "option",
                            "in": {
                                "_id": "$$option._id",
                                "text": "$$option.text",
                                "votes_count": "$$option.votes_count",
                                "voters": "$$option.voters"
                            }
                        }
                    }
                }
            }
        ];
    
        let mut cursor = self.collection.aggregate(pipeline).await?;
        let mut results = Vec::new();
    
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }
    
        Ok(results)
    }
    
    pub async fn get_closed_polls(
        &self, 
        page: u64, 
        per_page: u64
    ) -> Result<Vec<Document>, mongodb::error::Error> {
        // Validate pagination parameters
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);
        
        // Calculate skip for pagination
        let skip = (page - 1) * per_page;
    
        let pipeline = vec![
            // Match only closed polls
            doc! {
                "$match": {
                    "is_open": false
                }
            },
            // First lookup to expand options
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "expanded_options"
                }
            },
            // Second lookup for poll owner details
            doc! {
                "$lookup": {
                    "from": "users",
                    "localField": "owner_id",
                    "foreignField": "_id",
                    "as": "owner"
                }
            },
            // Unwind owner array (since it will be single document)
            doc! {
                "$unwind": {
                    "path": "$owner",
                    "preserveNullAndEmptyArrays": true
                }
            },
            // For each option, expand its voters
            doc! {
                "$lookup": {
                    "from": "users",
                    "let": { "expanded_options": "$expanded_options" },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$expanded_options.voters"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": 1,
                                "username": 1,
                                "uuid": 1
                            }
                        }
                    ],
                    "as": "voter_details"
                }
            },
            // Map voters to their respective options
            doc! {
                "$addFields": {
                    "expanded_options": {
                        "$map": {
                            "input": "$expanded_options",
                            "as": "option",
                            "in": {
                                "$mergeObjects": [
                                    "$$option",
                                    {
                                        "voters": {
                                            "$map": {
                                                "input": {
                                                    "$filter": {
                                                        "input": "$voter_details",
                                                        "as": "voter",
                                                        "cond": {
                                                            "$in": ["$$voter._id", "$$option.voters"]
                                                        }
                                                    }
                                                },
                                                "as": "voter",
                                                "in": {
                                                    "_id": "$$voter._id",
                                                    "username": "$$voter.username",
                                                    "uuid": "$$voter.uuid"
                                                }
                                            }
                                        }
                                    }
                                ]
                            }
                        }
                    }
                }
            },
            // Calculate total votes
            doc! {
                "$addFields": {
                    "total_votes": {
                        "$sum": "$expanded_options.votes_count"
                    }
                }
            },
            // Sort by total votes in descending order
            doc! {
                "$sort": {
                    "total_votes": -1
                }
            },
            // Pagination
            doc! {
                "$skip": skip as i32
            },
            doc! {
                "$limit": per_page as i32
            },
            // Final projection
            doc! {
                "$project": {
                    "_id": 1,
                    "id": 1,
                    "title": 1,
                    "total_votes": 1,
                    "is_open": 1,
                    "owner_username": "$owner.username",
                    "options": {
                        "$map": {
                            "input": "$expanded_options",
                            "as": "option",
                            "in": {
                                "_id": "$$option._id",
                                "text": "$$option.text",
                                "votes_count": "$$option.votes_count",
                                "voters": "$$option.voters"
                            }
                        }
                    }
                }
            }
        ];
    
        let mut cursor = self.collection.aggregate(pipeline).await?;
        let mut results = Vec::new();
    
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }
    
        Ok(results)
    }

    
    pub async fn count_live_polls(&self) -> Result<u64, mongodb::error::Error> {
        self.collection.count_documents(doc! {"is_open": true}).await
    }

    pub async fn count_closed_polls(&self) -> Result<u64, mongodb::error::Error> {
        self.collection.count_documents(doc! {"is_open": false}).await
    }

}

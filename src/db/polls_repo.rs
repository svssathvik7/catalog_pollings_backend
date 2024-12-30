use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, oid::ObjectId, Document},
    results::InsertOneResult,
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::DB;

#[derive(Deserialize, Serialize, Debug)]
pub struct Poll {
    pub id: String,
    pub title: String,
    pub owner_id: String,
    pub options: Vec<ObjectId>,
    pub is_open: bool,
    pub voters: Vec<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

pub struct PollRepo {
    pub collection: Collection<Poll>,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PollResponse {
    pub poll: Option<Poll>,
    #[serde(rename="camelCase")]
    pub has_voted: bool
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

    pub async fn get(
        &self,
        poll_id: &str,
        username: &str,
    ) -> Result<PollResponse, mongodb::error::Error> {
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

        if let Some(doc) = cursor.try_next().await? {
            // Deserialize the document into a Poll struct
            let poll: Poll = bson::from_document(doc)?;
    
            // Check if the username is in the voters list
            let has_voted = poll.voters.iter().any(|voter| voter == username);
    
            let poll_response = PollResponse {
                poll: Some(poll),
                has_voted,
            };
    
            Ok(poll_response)
        } else {

            Ok(PollResponse{poll:None,has_voted: false})
        }
    }

    pub async fn is_owner(&self, poll_id: &str, username: &str) -> bool {
        match self.get(poll_id, username).await {
            Ok(poll_response) => {
                let poll = match poll_response.poll{
                    Some(poll) => poll,
                    None => {
                        return false;
                    }
                };
                return poll.owner_id == username;
            }
            Err(_) => false,
        }
    }
    

    pub async fn add_vote(
        &self,
        poll_id: &str,
        username: String,
        option_id: ObjectId,
        db: &DB,
    ) -> Result<bool, mongodb::error::Error> {
        let mut session = db.client.start_session().await.unwrap();
        session.start_transaction().await.unwrap();
        // 1. Fetch poll details
        let poll_doc = match self.get(poll_id, &username).await {
            Ok(poll_response) => {
                match poll_response.poll {
                    Some(poll) => poll,
                    None => {return Ok(false);}
                }
            },
            Err(e)=>{
                return Ok(false);
            }
        };

        // 2. Check poll status
        let is_open = poll_doc.is_open;
        if !is_open {
            session.abort_transaction().await.unwrap();
            return Ok(false); // Poll is closed
        }

        // 3. Validate if option belongs to this poll
        let poll_options: Vec<ObjectId> = poll_doc
            .options;

        if !poll_options.contains(&option_id) {
            session.abort_transaction().await.unwrap();
            return Ok(false); // Option not part of this poll
        }

        // 4. Check if user has already voted in this poll
        let voters: Vec<String> = poll_doc.voters;

        if voters.contains(&username) {
            session.abort_transaction().await.unwrap();
            return Ok(false); // User already voted
        }

        // 5. Prepare update operations
        let poll_filter = doc! {"id": poll_id};
        let poll_update = doc! {
            "$addToSet": {"voters": &username},
        };

        let poll_update_result = self.collection.update_one(poll_filter, poll_update).await?;
        if poll_update_result.matched_count == 0 {
            return Ok(false); // Poll update failed
        }

        let option_filter = doc! {"id": option_id};
        let option_update = doc! {
            "$add": {"votes": 1}
        };

        let option_poll_result = db
            .options
            .collection
            .update_one(option_filter, option_update)
            .await?;
        session.commit_transaction().await.unwrap();
        Ok(true)
    }

    pub async fn close_poll(
        &self,
        poll_id: &str,
        username: &str,
    ) -> Result<bool, mongodb::error::Error> {
        if !self.is_owner(poll_id, username).await {
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

    pub async fn reset_poll(
        &self,
        poll_id: &str,
        db: &DB,
        username: &str,
    ) -> Result<bool, mongodb::error::Error> {
        if !self.is_owner(poll_id, username).await {
            return Ok(false);
        }
        let poll_match = match self.get(poll_id, username).await {
            Ok(poll_response) => {
                match poll_response.poll{
                    Some(poll) => poll,
                    None => {
                        return Ok(false);
                    }
                }
            },
            Err(e) => {
                return Ok(false);
            }
        };
        let options_ids: Vec<ObjectId> = poll_match
            .options;

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
        per_page: u64,
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
            },
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
        per_page: u64,
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
            },
        ];

        let mut cursor = self.collection.aggregate(pipeline).await?;
        let mut results = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }

        Ok(results)
    }

    pub async fn count_live_polls(&self) -> Result<u64, mongodb::error::Error> {
        self.collection
            .count_documents(doc! {"is_open": true})
            .await
    }

    pub async fn count_closed_polls(&self) -> Result<u64, mongodb::error::Error> {
        self.collection
            .count_documents(doc! {"is_open": false})
            .await
    }
}

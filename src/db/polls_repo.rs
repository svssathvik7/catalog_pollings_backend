use std::{error::Error, str};

use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, oid::ObjectId, Document},
    results::InsertOneResult,
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use crate::models::poll_api_model::{PollOptionResult, PollResults};

use super::{options_repo::OptionModel, DB};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPollResponse {
    pub id: String,
    pub title: String,
    pub owner_id: String,
    pub options: Vec<OptionModel>,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct PollResponse {
    pub poll: Option<GetPollResponse>,
    pub has_voted: bool,
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

    pub async fn delete(
        &self,
        poll_id: &str,
        username: &str,
    ) -> Result<bool, mongodb::error::Error> {
        // Check if the user is the owner of the poll
        if !self.is_owner(poll_id, username).await {
            return Ok(false); // Return false if the user is not the owner
        }

        // Build the query to find the poll by ID
        let query = doc! { "id": poll_id };

        // Attempt to delete the poll
        match self.collection.delete_one(query).await {
            Ok(delete_result) => {
                if delete_result.deleted_count > 0 {
                    Ok(true) // Return true if a document was successfully deleted
                } else {
                    Ok(false) // Return false if no document matched the query
                }
            }
            Err(e) => {
                eprintln!("Error deleting poll: {:?}", e); // Log the error
                Err(e) // Propagate the error to the caller
            }
        }
    }

    pub async fn get(
        &self,
        poll_id: &str,
        username: &str,
    ) -> Result<PollResponse, mongodb::error::Error> {
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
                    "options": 1,
                    "is_open": 1,
                    "created_at": 1,
                    "updated_at": 1,
                    "voters": 1,
                    "id": 1
                }
            },
        ];
        let mut cursor = self.collection.aggregate(pipeline).await?;

        if let Some(doc) = cursor.try_next().await? {
            // Deserialize the document into a Poll struct
            let poll: GetPollResponse = bson::from_document(doc)?;
            let mut has_voted: bool = true;
            if username.is_empty() {
                has_voted = true;
            } else {
                // Check if the username is in the voters list
                has_voted = poll.voters.iter().any(|voter| voter == username);
            }

            let poll_response = PollResponse {
                poll: Some(poll),
                has_voted,
            };
            // println!("{:?}", poll_response);

            Ok(poll_response)
        } else {
            Ok(PollResponse {
                poll: None,
                has_voted: false,
            })
        }
    }

    pub async fn is_owner(&self, poll_id: &str, username: &str) -> bool {
        match self.get(poll_id, username).await {
            Ok(poll_response) => {
                let poll = match poll_response.poll {
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
            Ok(poll_response) => match poll_response.poll {
                Some(poll) => poll,
                None => {
                    return Ok(false);
                }
            },
            Err(e) => {
                eprintln!("Error fetching poll: {:?}", e);
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
        let poll_options = poll_doc.options;

        let option_exists = poll_options.iter().any(|option| option._id == option_id);

        if !option_exists {
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

        let option_filter = doc! {"_id": option_id};
        let option_update = doc! {
            "$inc": {"votes_count": 1}
        };

        let _option_poll_result = db
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
            .update_one(filter, doc! {"$set" : {"is_open": false}})
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
            Ok(poll_response) => match poll_response.poll {
                Some(poll) => poll,
                None => {
                    return Ok(false);
                }
            },
            Err(e) => {
                eprintln!("Error resetting poll! {:?}", e);
                return Ok(false);
            }
        };
        let options = poll_match.options;

        for option in options {
            let filter = doc! {"_id": option._id};
            let update = doc! {"$set": {"votes_count": 0}};
            db.options.collection.update_one(filter, update).await?;
        }

        let filter = doc! {"id": poll_id};
        let update = doc! {
            "$set": {
                "is_open": true,
                "voters": Vec::<ObjectId>::new()
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
                    "as": "options"
                }
            },
            doc! {
                "$addFields": {
                    "total_voters": { "$size": "$voters" }
                }
            },
            // Sort by total_voters in descending order
            doc! {
                "$sort": {
                    "total_voters": -1
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
                    "is_open": 1,
                    "voters": 1,
                    "owner_id": "$owner_id",
                    "options": {
                        "$map": {
                            "input": "$options",
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
                    "as": "options"
                }
            },
            doc! {
                "$addFields": {
                    "total_voters": { "$size": "$voters" }
                }
            },
            // Sort by total_voters in descending order
            doc! {
                "$sort": {
                    "total_voters": -1
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
                    "voters": 1,
                    "is_open": 1,
                    "owner_id": 1,
                    "options": {
                        "$map": {
                            "input": "$options",
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
    pub async fn get_polls_by_username(
        &self,
        username: &str,
        page: u64,
        per_page: u64,
        sort_by: &str,
        sort_order: i8,
    ) -> Result<Vec<Document>, mongodb::error::Error> {
        // Validate pagination parameters - keeping them reasonable
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);

        // Calculate skip value for pagination
        let skip = (page - 1) * per_page;

        let sort_direction = if sort_order >= 0 { 1 } else { -1 };

        let sort_doc = match sort_by {
            "votes" => doc! {
                "$sort": {
                    "total_votes": sort_direction
                }
            },
            "created_at" => doc! {
                "$sort": {
                    "created_at": sort_direction
                }
            },
            "updated_at" => doc! {
                "$sort": {
                    "updated_at": sort_direction
                }
            },
            "title" => doc! {
                "$sort": {
                    "title": sort_direction
                }
            },
            // Default to created_at if sort_by is not recognized
            _ => doc! {
                "$sort": {
                    "created_at": -1
                }
            },
        };

        // Create the aggregation pipeline
        let pipeline = vec![
            // Match polls owned by the specified username
            doc! {
                "$match": {
                    "owner_id": username
                }
            },
            // Lookup to expand the options
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "options"
                }
            },
            // Calculate total votes
            doc! {
                "$addFields": {
                    "total_votes": {
                        "$sum": "$options.votes_count"
                    }
                }
            },
            sort_doc,
            // Apply pagination
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
                    "voters": 1,
                    "is_open": 1,
                    "created_at": 1,
                    "updated_at": 1,
                    "owner_id": 1,
                    "options": {
                        "$map": {
                            "input": "$options",
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

        // Execute the aggregation
        let mut cursor = self.collection.aggregate(pipeline).await?;
        let mut results = Vec::new();

        // Collect all documents
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }

        Ok(results)
    }

    // Helper function to count total polls by username
    pub async fn count_polls_by_username(
        &self,
        username: &str,
    ) -> Result<u64, mongodb::error::Error> {
        self.collection
            .count_documents(doc! {"owner_id": username})
            .await
    }

    pub async fn get_poll_results(
        &self,
        poll_id: &str,
    ) -> Result<Option<PollResults>, Box<dyn Error>> {
        // Create an aggregation pipeline to get poll details with options
        let pipeline = vec![
            // Match the specific poll
            doc! {
                "$match": {
                    "id": poll_id
                }
            },
            // Lookup to get the options
            doc! {
                "$lookup": {
                    "from": "options",
                    "localField": "options",
                    "foreignField": "_id",
                    "as": "options"
                }
            },
            // Calculate total votes across all options
            doc! {
                "$addFields": {
                    "total_votes": {
                        "$sum": "$options.votes_count"
                    }
                }
            },
            // Project the final format
            doc! {
                "$project": {
                    "_id": 0,
                    "id": 1,
                    "total_votes": 1,
                    "title": 1,
                    "options": {
                        "$map": {
                            "input": "$options",
                            "as": "option",
                            "in": {
                                "text": "$$option.text",
                                "votes_count": "$$option.votes_count",
                                "votes_percentage": {
                                    "$cond": [
                                        { "$eq": ["$total_votes", 0] },
                                        0.0,
                                        {
                                            "$multiply": [
                                                { "$divide": ["$$option.votes_count", "$total_votes"] },
                                                100
                                            ]
                                        }
                                    ]
                                }
                            }
                        }
                    }
                }
            },
        ];

        // Execute the aggregation pipeline
        let mut cursor = self.collection.aggregate(pipeline).await?;

        // Get the first (and should be only) result
        if let Some(doc) = cursor.try_next().await? {
            println!("{:?}", doc);
            // Convert BSON document to our PollResults structure
            let id = doc.get_str("id")?.to_string();
            let title = doc.get_str("title")?.to_string();
            let total_votes = doc.get_i64("total_votes")?;

            let options_array = doc.get_array("options")?;
            let mut options = Vec::new();

            for option_doc in options_array {
                if let bson::Bson::Document(option) = option_doc {
                    options.push(PollOptionResult {
                        text: option.get_str("text")?.to_string(),
                        votes_count: option.get_i64("votes_count")?,
                        votes_percentage: option.get_f64("votes_percentage")?,
                    });
                }
            }
            println!("its last");

            Ok(Some(PollResults {
                id,
                title,
                options,
                total_votes,
            }))
        } else {
            Ok(None)
        }
    }
}

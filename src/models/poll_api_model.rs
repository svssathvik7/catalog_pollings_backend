use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::options_repo::OptionModel;

#[derive(Deserialize, Serialize, Debug)]
pub struct NewPollRequest {
    pub title: String,
    pub options: Vec<OptionRequest>,
    pub ownername: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OptionRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PollOptionResult {
    pub text: String,
    pub votes_count: i64,
    pub votes_percentage: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PollResults {
    pub id: String,
    pub title: String,
    pub total_votes: i64,
    pub options: Vec<PollOptionResult>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPollResponse {
    pub id: String,
    pub title: String,
    pub owner_id: String,
    pub options: Vec<OptionModel>,
    pub total_votes: i64,
    pub is_open: bool,
    #[serde(skip_serializing)]
    pub voters: Vec<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PollResponse {
    pub poll: Option<GetPollResponse>,
    pub has_voted: bool,
}

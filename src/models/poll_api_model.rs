use serde::{Deserialize, Serialize};

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

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct NewPollRequest {
    pub title: String,
    pub options: Vec<OptionRequest>,
    pub ownername: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OptionRequest{
    pub text: String
}
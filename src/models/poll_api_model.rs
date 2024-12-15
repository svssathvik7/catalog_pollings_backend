use serde::{Deserialize, Serialize};

use crate::db::options_repo::Option;

#[derive(Deserialize, Serialize, Debug)]
pub struct NewPollRequest {
    pub title: String,
    pub options: Vec<Option>,
    pub ownername: String,
}

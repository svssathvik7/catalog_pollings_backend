use mongodb::{results::InsertOneResult, Collection, Database};
use serde::{Deserialize, Serialize};

use super::users_repo::User;

#[derive(Deserialize,Serialize,Debug)]
pub struct Option{
    pub text: String,
    pub votes_count: u64,
    pub voters: Vec<User>
}

pub struct OptionRepo{
    pub collection: Collection<Option>
}
impl OptionRepo{
    pub fn init(db: &Database) -> Self{
        let options_repo = db.collection("options");
        Self{
            collection: options_repo
        }
    }

    pub async fn insert(&self,new_option: Option) -> Result<InsertOneResult,mongodb::error::Error>{
        let result = self.collection.insert_one(new_option).await;
        result
    }
}
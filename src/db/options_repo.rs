use mongodb::{
    bson::{oid::ObjectId, Document},
    results::{DeleteResult, InsertOneResult},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Option {
    pub text: String,
    pub votes_count: u64,
}

pub struct OptionRepo {
    pub collection: Collection<Option>,
}
impl OptionRepo {
    pub async fn init(db: &Database) -> Self {
        let options_repo = db.collection("options");
        Self {
            collection: options_repo,
        }
    }

    pub async fn insert(
        &self,
        new_option: Option,
    ) -> Result<InsertOneResult, mongodb::error::Error> {
        let result = self.collection.insert_one(new_option).await;
        result
    }

    pub async fn delete(&self, filter: Document) -> Result<DeleteResult, mongodb::error::Error> {
        let result = self.collection.delete_one(filter).await;
        result
    }
}

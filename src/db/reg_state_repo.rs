use mongodb::{Collection, Database};

use super::DB;

pub struct RegState {
    pub username: String,
    pub uuid: String,
    pub reg_state: serde_json::Value,
}

pub struct RegStateRepo {
    collection: Collection<RegState>,
}

impl RegStateRepo {
    pub async fn init(db: &Database) -> Self {
        let reg_state_collection: Collection<RegState> = db.collection("reg_states");
        Self {
            collection: reg_state_collection,
        }
    }
}

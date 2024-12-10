use mongodb::{Collection, Database};

pub struct User{
    pub username: String,
    pub uuid: String,
    pub sk: serde_json::Value
}

pub struct UserRepo{
    collection: Collection<User>
}

impl UserRepo{
    pub async fn init(db: &Database) -> Self{
        let users_collection = db.collection("users");
        Self{
            collection: users_collection
        }
    }
}
use rocket::http::Status;
use crate::lichess::client::lichess_user;
use crate::models::{User};

#[get("/api/lichess/user/<username>")]
pub async fn lichess_user_endpoint(_user: User, username: String) -> Result<String, Status> {
    lichess_user(&username).await
}
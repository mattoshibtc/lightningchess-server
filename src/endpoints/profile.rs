use crate::models::{User, UserProfile};

#[get("/api/profile")]
pub async fn profile(user: User) -> String {
    let user_profile: UserProfile = UserProfile {
        username: user.username
    };
    serde_json::to_string(&user_profile).unwrap()
}
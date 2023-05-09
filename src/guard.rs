
pub mod auth {
    use hyper::StatusCode;
    use moka::future::Cache;
    use reqwest::Client;
    use rocket::http::Status;
    use rocket::outcome::Outcome::{Failure};
    use rocket::{Request, State};
    use rocket::outcome::{try_outcome};
    use rocket::request::{FromRequest, Outcome};
    use crate::models::{Account, User};

    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for User {
        type Error = ();
        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
            let cache = try_outcome!(request.guard::<&State<Cache<String, User>>>().await);

            let access_token = request.cookies().get("llchess_access_token").map(|c| c.value());
            match access_token {
                Some(token) => {
                    let maybe_user = cache.get(token);
                    match maybe_user {
                        Some(u) => {
                            return Outcome::Success(u)
                        },
                        None => println!("Cache miss")
                    }
                    let bearer = format!("Bearer {token}");
                    let response = Client::new()
                        .get("https://lichess.org/api/account")
                        .header("Authorization", bearer)
                        .send().await;
                    match response {
                        Ok(res) => {
                            println!("Status: {}", res.status());
                            println!("Headers:\n{:#?}", res.headers());
                            if res.status() == StatusCode::TOO_MANY_REQUESTS {
                                return Failure((Status::TooManyRequests,()))
                            };
                            let text = res.text().await;
                            match text {
                                Ok(text) => {
                                    let account: Account = serde_json::from_str(&text).unwrap();
                                    cache.insert(token.to_string(), User { access_token: token.to_string(), username: account.username.to_string()}).await;
                                    Outcome::Success(User { access_token: token.to_string(), username: account.username})
                                }
                                Err(e) => {
                                    println!("error in text():\n{}", e);
                                    Outcome::Forward(())
                                }
                            }
                        },
                        Err(e) => {
                            println!("error from api/account:\n{}", e);
                            Outcome::Forward(())
                        }
                    }
                }
                None => {
                    println!("no access token\n");
                    Outcome::Forward(())
                }
            }
        }
    }

}
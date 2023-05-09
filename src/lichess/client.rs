use reqwest::Client;
use rocket::http::Status;
use crate::models::{Challenge, LichessAcceptChallengeResponse, LichessAddTimeResponse, LichessChallenge, LichessChallengeClock, LichessChallengeResponse, LichessUser, User};

fn parse_to_lichess_challenge(challenge: &Challenge) -> LichessChallenge {
    let color = match challenge.color.as_deref() {
        Some("white") => "black".to_string(),
        Some("black") => "white".to_string(),
        _ => "".to_string()
    };

    let time_limit = challenge.time_limit.unwrap();
    let opponent_time_limit = challenge.opponent_time_limit.unwrap();
    let limit = if time_limit < opponent_time_limit { time_limit} else {opponent_time_limit};
    return LichessChallenge {
        rated: true,
        clock: LichessChallengeClock {
            limit: limit.to_string(),
            increment: challenge.increment.unwrap().to_string(),
        },
        color,
        variant: "standard".to_string(),
        rules: "noClaimWin".to_string(),
    };
}

// programmatically accept challenge for person who created challenge
pub async fn accept_lichess_challenge(challenge: &Challenge, lichess_challenge_response: &LichessChallengeResponse) -> Result<bool, Status> {
    let url = format!("https://lichess.org/api/challenge/{}/accept", lichess_challenge_response.challenge.id);
    let bearer = format!("Bearer {}", &challenge.challenger_token.as_ref().unwrap());
    let resp = Client::new()
        .post(url)
        .header("Authorization", bearer)
        .send().await;
    return match resp {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());

            let text = res.text().await;
            match text {
                Ok(text) => {
                    println!("text!: {}", text);
                    let lichess_accept_challenge_response: LichessAcceptChallengeResponse = serde_json::from_str(&text).unwrap();
                    if lichess_accept_challenge_response.ok {
                        Ok(true)
                    } else {
                        Err(Status::InternalServerError)
                    }
                }
                Err(e) => {
                    println!("error: {}", e);
                    Err(Status::InternalServerError)
                }
            }
        },
        Err(e) => {
            println!("error challenge accept on lichess: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn add_time(user: &User, challenge: &Challenge, lichess_challenge_response: &LichessChallengeResponse) -> Result<bool, Status> {
    let time_limit = challenge.time_limit.unwrap();
    let opponent_time_limit = challenge.opponent_time_limit.unwrap();
    if time_limit == opponent_time_limit {
        return Ok(true)
    }
    let time_to_add = (time_limit - opponent_time_limit).abs();
    let token = if time_limit < opponent_time_limit {
        &challenge.challenger_token.as_ref().unwrap()
    } else {
        &user.access_token
    };

    let url = format!("https://lichess.org/api/round/{}/add-time/{}", lichess_challenge_response.challenge.id, time_to_add.to_string());
    let bearer = format!("Bearer {token}");
    let resp = Client::new()
        .post(url)
        .header("Authorization", bearer)
        .send().await;

    return match resp {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());

            let text = res.text().await;
            match text {
                Ok(text) => {
                    println!("text!: {}", text);
                    let lichess_add_time_response: LichessAddTimeResponse = serde_json::from_str(&text).unwrap();
                    if lichess_add_time_response.ok {
                        Ok(true)
                    } else {
                        Err(Status::InternalServerError)
                    }
                }
                Err(e) => {
                    println!("error: {}", e);
                    Err(Status::InternalServerError)
                }
            }
        },
        Err(e) => {
            println!("error adding time on lichess: {}", e);
            Err(Status::InternalServerError)
        }
    }
}


pub async fn create_lichess_challenge(user: &User, challenge: &Challenge) -> Result<LichessChallengeResponse, Status> {
    let url = format!("https://lichess.org/api/challenge/{}", &challenge.username);
    let access_token = &user.access_token;
    let bearer = format!("Bearer {access_token}");
    let body = parse_to_lichess_challenge(&challenge);
    let resp = Client::new()
        .post(url)
        .json(&body)
        .header("Authorization", bearer)
        .send().await;

    let lichess_challenge_response: LichessChallengeResponse = match resp {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());

            let text = res.text().await;
            match text {
                Ok(text) => {
                    println!("text!: {}", text);
                    serde_json::from_str(&text).unwrap()
                }
                Err(e) => {
                    println!("error: {}", e);
                    return Err(Status::InternalServerError)
                }
            }
        },
        Err(e) => {
            println!("error creating on lichess: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    Ok(lichess_challenge_response)
}

pub async fn lichess_user(username: &str) -> Result<String, Status> {
    let url = format!("https://lichess.org/api/user/{username}");
    println!("url: {url}");
    let response = Client::new()
        .get(url)
        .send().await;
    match response {
        Ok(res) => {
            if res.status() == 404 {
                return Err(Status::NotFound)
            }
            return match res.text().await {
                Ok(t) => {
                    println!("text!: {}", t);
                    let lichess_user: LichessUser = serde_json::from_str(&t).unwrap();
                    Ok(serde_json::to_string(&lichess_user).unwrap())
                }
                Err(e) => {
                    println!("error: {}", e);
                    Err(Status::InternalServerError)
                }
            }
        },
        Err(_) => Err(Status::InternalServerError)
    }
}
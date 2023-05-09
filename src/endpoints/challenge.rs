use rocket::http::{Status};
use rocket::State;
use crate::models::{Balance, Challenge, ChallengeAcceptRequest, Transaction, User};
use sqlx::Postgres;
use sqlx::Pool;
use crate::errors::{LightningChessResult, ParseRequestError};
use crate::lichess::client::{accept_lichess_challenge, add_time, create_lichess_challenge};

fn parse_request_to_challenge(challenge_request: &str) -> LightningChessResult<Challenge> {
    let challenge: Challenge = serde_json::from_str(&challenge_request)?;

    let time_limit = challenge.time_limit.ok_or::<ParseRequestError>(ParseRequestError { m: "time_limit required".to_string()})?;
    let opp_time_limit = challenge.opponent_time_limit.ok_or::<ParseRequestError>(ParseRequestError {m: "opp_time_limit required".to_string()})?;
    if time_limit < 60 || time_limit > 600 || opp_time_limit < 60 || opp_time_limit > 600 || time_limit % 15 != 0 || opp_time_limit % 15 != 0 {
        return Err(ParseRequestError { m: "time limit constraint".to_string()}.into())
    }

    let increment = challenge.increment.ok_or::<ParseRequestError>(ParseRequestError {m: "increment required".to_string()})?;
    if increment < 0 || increment > 5 {
        return Err(ParseRequestError { m: "increment constraint".to_string()}.into())
    }

    let sats = challenge.sats.ok_or::<ParseRequestError>(ParseRequestError {m: "sats required".to_string()})?;
    if sats < 100 || sats > 3_000_000 {
        return Err(ParseRequestError {m: "sats constraint".to_string()}.into())
    }

    let color = challenge.color.as_ref().ok_or::<ParseRequestError>(ParseRequestError {m: "color required".to_string()})?;
    if color != "white" && color != "black" {
        return Err(ParseRequestError {m: "color constraint".to_string()}.into())
    }

    Ok(challenge)
}

#[post("/api/challenge", data = "<challenge_request>")]
pub async fn create_challenge(user: User, pool: &State<Pool<Postgres>>, challenge_request: String) -> Result<String, Status> {
    println!("challenge request!: {}", challenge_request);
    let challenge_result = parse_request_to_challenge(&challenge_request);
    let challenge = match challenge_result {
        Ok(c) => c,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::BadRequest)
        }
    };

    // only allow creation of challenge if user has enough funds
    let balance_result = sqlx::query_as::<_,Balance>( "SELECT * FROM lightningchess_balance WHERE username=$1")
        .bind(&user.username)
        .fetch_one(&**pool).await;
    match balance_result {
        Ok(balance) => {
            if balance.balance < 0 || balance.balance < challenge.sats.unwrap() {
                return Err(Status::InternalServerError)
            }
        },
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::InternalServerError)
        }
    }

    //create transaction
    let tx_result = pool.begin().await;
    let mut tx = match tx_result {
        Ok(t) => t,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    // save challenge to db
    let status = "WAITING FOR ACCEPTANCE";
    let challenge_result = sqlx::query_as::<_,Challenge>("INSERT INTO challenge (username, time_limit, opponent_time_limit, increment, color, sats, opp_username, status, expire_after, challenger_token) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *")
        .bind(&user.username)
        .bind(&challenge.time_limit)
        .bind(&challenge.opponent_time_limit)
        .bind(&challenge.increment)
        .bind(&challenge.color)
        .bind(&challenge.sats)
        .bind(&challenge.opp_username)
        .bind(&status)
        .bind(1800) // default to 30min expiry
        .bind(&user.access_token)
        .fetch_one(&mut tx).await;

    let challenge_json_result = match challenge_result {
        Ok(r) => {
            Ok(serde_json::to_string(&r).unwrap())
        },
        Err(e) => {
            println!("insert challenge error: {}", e);
            return Err(Status::InternalServerError)
        }
    };
    challenge_json_result
}

#[post("/api/accept-challenge", data = "<challenge_accept_request>")]
pub async fn accept_challenge(user: User, pool: &State<Pool<Postgres>>, challenge_accept_request: String) -> Result<String, Status> {
    println!("challenge_accept_request!: {}", challenge_accept_request);
    let challenge_accept_request_result: Result<ChallengeAcceptRequest, serde_json::Error> = serde_json::from_str(&challenge_accept_request);
    let challenge_accept_request = match challenge_accept_request_result {
        Ok(c) => c,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::BadRequest)
        }
    };

    let challenge_result = sqlx::query_as::<_,Challenge>( "SELECT * FROM challenge WHERE id=$1")
        .bind(challenge_accept_request.id)
        .fetch_one(&**pool).await;

    let challenge = match challenge_result {
        Ok(c) => c,
        Err(e) => {
            println!("error getting challenge in challenge accept: {}", e.as_database_error().unwrap().message());
            return Err(Status::InternalServerError)
        }
    };

    // only opponent can accept the challenge and challenge must be in correct status
    if challenge.opp_username != user.username || challenge.status.as_ref().unwrap() != "WAITING FOR ACCEPTANCE" {
        return Err(Status::BadRequest)
    }

    // only allow accept of challenge if user has enough funds
    let balance_result = sqlx::query_as::<_,Balance>( "SELECT * FROM lightningchess_balance WHERE username=$1")
        .bind(&user.username)
        .fetch_one(&**pool).await;
    match balance_result {
        Ok(balance) => {
            if balance.balance < 0 || balance.balance < challenge.sats.unwrap() {
                return Err(Status::InternalServerError)
            }
        },
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::InternalServerError)
        }
    }

    //create transaction
    let tx_result = pool.begin().await;
    let mut tx = match tx_result {
        Ok(t) => t,
        Err(e) => {
            println!("error creating tx: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    // deduct balance
    let balance_result = sqlx::query_as::<_,Balance>( "UPDATE lightningchess_balance SET balance=balance - $1 WHERE username=$2 RETURNING *")
        .bind(challenge.sats.unwrap())
        .bind(&user.username)
        .fetch_one(&mut tx).await;

    match balance_result {
        Ok(balance) => {
            if balance.balance < 0 {
                println!("balance is less than 0");
                return Err(Status::InternalServerError)
            }
            println!("updated balance")
        },
        Err(e) => {
            println!("error updating balance: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    // insert transaction into transaction db
    let ttype = "accept challenge";
    let detail = format!("challenge vs {}", challenge.username);
    let state = "SETTLED";
    let transaction_result = sqlx::query_as::<_,Transaction>("INSERT INTO lightningchess_transaction (username, ttype, detail, amount, state) VALUES ($1, $2, $3, $4, $5) RETURNING *")
        .bind(&user.username)
        .bind(ttype)
        .bind(&detail)
        .bind(-challenge.sats.unwrap())
        .bind(state)
        .fetch_one(&mut tx).await;

    match transaction_result {
        Ok(_) => println!("successfully inserted transaction"),
        Err(e) => {
            println!("error inserting tx: {}", e);
            return Err(Status::InternalServerError)
        }
    }

    println!("create challenge lichess challenge");
    let lichess_challenge_response = create_lichess_challenge(&user, &challenge).await?;
    println!("accept challenge lichess");
    accept_lichess_challenge(&challenge, &lichess_challenge_response).await?;
    println!("add time lichess");
    add_time(&user, &challenge, &lichess_challenge_response).await?;

    // update challenge in db
    let status = "ACCEPTED";
    let pg_query_result = sqlx::query_as::<_,Challenge>("UPDATE challenge SET status=$1, lichess_challenge_id=$2 WHERE id=$3 RETURNING *")
        .bind(status)
        .bind(&lichess_challenge_response.challenge.id)
        .bind(&challenge_accept_request.id)
        .fetch_one(&mut tx).await;

    let challenge_json_result = match pg_query_result {
        Ok(r) => {
            Ok(serde_json::to_string(&r).unwrap())
        },
        Err(e) => {
            println!("update challenge in challenge accept: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    // commit transaction, return challenge
    let commit_result = tx.commit().await;
    return match commit_result {
        Ok(_) => {
            challenge_json_result
        },
        Err(e) => {
            println!("error committing: {}", e);
            Err(Status::InternalServerError)
        }
    }

}
#[get("/api/challenges")]
pub async fn challenges(user: User, pool: &State<Pool<Postgres>>) -> Result<String, Status> {
    let challenges = sqlx::query_as::<_,Challenge>( "SELECT * FROM challenge WHERE username=$1 OR opp_username=$1 ORDER BY created_on DESC LIMIT 100")
        .bind(&user.username)
        .fetch_all(&**pool).await;
    match challenges {
        Ok(challenges) => Ok(serde_json::to_string(&challenges).unwrap()),
        Err(e) => {
            println!("error: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[get("/api/challenge/<challenge_id>")]
pub async fn lookup_challenge(user: User, pool: &State<Pool<Postgres>>, challenge_id: String) -> Result<String, Status> {
    let challenge_id_int = match challenge_id.parse::<i32>() {
        Ok(i) => i,
        Err(_) => return Err(Status::BadRequest)
    };
    let challenge = sqlx::query_as::<_,Challenge>( "SELECT * FROM challenge WHERE id=$1")
        .bind(challenge_id_int)
        .fetch_one(&**pool).await;

    match challenge {
        Ok(challenge) =>  {
            // only be able to look up own games
            if challenge.username != user.username && challenge.opp_username != user.username {
                Err(Status::Unauthorized)
            } else {
                Ok(serde_json::to_string(&challenge).unwrap())
            }
        },
        Err(e) => {
            println!("error: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_challenge() -> Challenge {
        Challenge {
            id: 1,
            username: "user1".to_string(),
            time_limit: Some(300),
            opponent_time_limit: Some(300),
            increment: Some(0),
            color: Some("white".to_string()),
            sats: Some(100),
            opp_username: "user2".to_string(),
            status: None,
            lichess_challenge_id: None,
            created_on: None,
            expire_after: None
        }
    }

    #[test]
    fn valid_challenge() {
        let challenge = get_challenge();
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_ok());
    }

    #[test]
    fn invalid_sat_low() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            sats: Some(99),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_sat_high() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            sats: Some(3_000_001),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_sat_none() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            sats: None,
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_color() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            color: Some("test".to_string()),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_color_none() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            color: None,
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_time_limit_low() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            time_limit: Some(45),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_time_limit_high() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            time_limit: Some(615),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }


    #[test]
    fn invalid_time_limit_none() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            time_limit: None,
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_time_limit_not_fifteen() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            time_limit: Some(74),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_opp_time_limit_low() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            opponent_time_limit: Some(30),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_opp_time_limit_high() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            opponent_time_limit: Some(615),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }


    #[test]
    fn invalid_opp_time_limit_none() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            opponent_time_limit: None,
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_opponent_time_limit_not_fifteen() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            opponent_time_limit: Some(74),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_increment_low() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            increment: Some(-1),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_increment_high() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            increment: Some(6),
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }


    #[test]
    fn invalid_increment_none() {
        let base_challenge = get_challenge();
        let challenge = Challenge {
            increment: None,
            ..base_challenge
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        let res = parse_request_to_challenge(&challenge_json);
        assert!(res.is_err());
    }

}
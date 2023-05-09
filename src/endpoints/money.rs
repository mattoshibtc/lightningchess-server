use rand::distributions::Alphanumeric;
use rand::Rng;
use rocket::http::Status;
use rocket::State;
use sqlx::{Pool, Postgres};
use crate::errors::{sqlx_err_to_status};
use crate::models::{Transaction, AddInvoiceRequest, User, Balance, SendPaymentRequest, SendPaymentResponse};
use crate::lightning::invoices::add_invoice;
use crate::lightning::payment::{decode_payment, make_payment};

#[post("/api/invoice", data = "<invoice_request_str>")]
pub async fn add_invoice_endpoint(user: User, pool: &State<Pool<Postgres>>, invoice_request_str: String) -> Result<String, Status> {
    println!("invoice request: {}", invoice_request_str);
    let invoice_request_result: Result<AddInvoiceRequest, serde_json::Error> = serde_json::from_str(&invoice_request_str);
    let invoice_request = match invoice_request_result {
        Ok(i) => i,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::BadRequest)
        }
    };

    // create preimage
    let preimage_bytes: Vec<u8>  = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .collect();

    // create memo
    let memo = format!("fund {} on lightningchess.io", &user.username);

    // create invoice
    let add_invoice_response_option = add_invoice(invoice_request.sats, &memo, preimage_bytes).await;
    let add_invoice_response = match add_invoice_response_option {
        Some(i) => i,
        None => return Err(Status::InternalServerError)
    };

    // save it to db
    let ttype = "invoice";
    let state = "OPEN";
    let pg_query_result = sqlx::query_as::<_,Transaction>("INSERT INTO lightningchess_transaction (username, ttype, detail, amount, state, payment_addr, payment_request) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *")
        .bind(&user.username)
        .bind(ttype)
        .bind(&memo)
        .bind(0) // default to zero until paid
        .bind(state)
        .bind(&add_invoice_response.payment_addr)
        .bind(&add_invoice_response.payment_request)
        .fetch_one(&**pool).await;

    return match pg_query_result {
        Ok(r) => {
            Ok(serde_json::to_string(&r).unwrap())
        },
        Err(e) => {
            println!("error: {}", e.as_database_error().unwrap().message());
            Err(Status::InternalServerError)
        }
    }
}

#[post("/api/transaction/<transaction_id>")]
pub async fn lookup_transaction(user: User, pool: &State<Pool<Postgres>>, transaction_id: String) -> Result<String, Status> {
    let transaction_id_int = match transaction_id.parse::<i32>() {
        Ok(i) => i,
        Err(_) => return Err(Status::BadRequest)
    };

    let transaction_result = sqlx::query_as::<_,Transaction>( "SELECT * FROM lightningchess_transaction WHERE transaction_id=$1")
        .bind(transaction_id_int)
        .fetch_one(&**pool).await;

    let transaction = match transaction_result {
        Ok(t) => t,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::InternalServerError)
        }
    };

    if transaction.username != user.username {
        return Err(Status::Unauthorized)
    }

    let transaction_result2 = sqlx::query_as::<_,Transaction>( "SELECT * FROM lightningchess_transaction WHERE transaction_id=$1")
        .bind(transaction_id_int)
        .fetch_one(&**pool).await;

    return match transaction_result2 {
        Ok(t2) => Ok(serde_json::to_string(&t2).unwrap()),
        Err(e) => {
            println!("error getting t2: {}", e);
            return Err(Status::InternalServerError)
        }
    }
}

#[get("/api/transactions")]
pub async fn transactions(user: User, pool: &State<Pool<Postgres>>) -> Result<String, Status> {
    let transactions = sqlx::query_as::<_,Transaction>( "SELECT * FROM lightningchess_transaction WHERE username=$1 ORDER BY transaction_id DESC LIMIT 100")
        .bind(&user.username)
        .fetch_all(&**pool).await;

    match transactions {
        Ok(t) => Ok(serde_json::to_string(&t).unwrap()),
        Err(e) => {
            println!("error: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[get("/api/balance")]
pub async fn balance(user: User, pool: &State<Pool<Postgres>>) -> Result<String, Status> {
    println!("getting balance for {}", &user.username);
    let balance_result = sqlx::query_as::<_,Balance>( "SELECT * FROM lightningchess_balance WHERE username=$1")
        .bind(&user.username)
        .fetch_optional(&**pool).await;
    match balance_result {
        Ok(balance_option) => {
            match balance_option {
                Some(balance) => Ok(serde_json::to_string(&balance).unwrap()),
                None => Ok(serde_json::to_string(&Balance {
                    balance_id: 0,
                    username: user.username,
                    balance: 0
                }).unwrap())
            }
        },
        Err(e) => {
            println!("error: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[post("/api/send-payment", data = "<send_payment_request_str>")]
pub async fn send_payment_endpoint(user: User, pool: &State<Pool<Postgres>>, send_payment_request_str: String) -> Result<String, Status> {
    println!("send_payment_request_str: {}", send_payment_request_str);
    let send_payment_result: Result<SendPaymentRequest, serde_json::Error> = serde_json::from_str(&send_payment_request_str);
    let send_payment = match send_payment_result {
        Ok(sp) => sp,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::BadRequest)
        }
    };

    // decode
    let decoded_option = decode_payment(&send_payment.payment_request).await;
    let decoded_payment = match decoded_option {
        Some(dp) => dp,
        None => return Err(Status::BadRequest)
    };
    let withdrawal_amt = decoded_payment.num_satoshis.parse::<i64>().unwrap();

    // not sure if this is possible
    if withdrawal_amt <= 0 {
        return Err(Status::BadRequest)
    }

    let mut tx = pool.begin().await.map_err(sqlx_err_to_status)?;

    let balance_result = sqlx::query_as::<_,Balance>( "SELECT * FROM lightningchess_balance WHERE username=$1 FOR UPDATE")
        .bind(&user.username)
        .fetch_one(&mut tx).await;
    let balance = match balance_result {
        Ok(b) => b,
        Err(e) => {
            println!("error: {}", e);
            return Err(Status::BadRequest)
        }
    };

    // only send if they have enough money
    if balance.balance < withdrawal_amt {
        return Err(Status::BadRequest)
    }

    let withdrawal_amt_neg = withdrawal_amt * -1;

    // insert payment into transactions table with status == 0PEN, commit
    // if we don't do this, we never have a way to retry if the update the db fails after the payment is made
    let withdrawal_ttype = "withdrawal";
    let withdrawal_detail = "";
    let withdrawal_state = "OPEN";
    let withdrawal_transaction_result = sqlx::query_as::<_, Transaction>( "INSERT INTO lightningchess_transaction (username, ttype, detail, amount, state, payment_hash) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *")
        .bind(&user.username)
        .bind(withdrawal_ttype)
        .bind(withdrawal_detail)
        .bind(&withdrawal_amt_neg)
        .bind(withdrawal_state)
        .bind(&decoded_payment.payment_hash)
        .fetch_one(&mut tx).await;

    let withdrawal_transaction = match withdrawal_transaction_result {
        Ok(t) => {
            println!("withdrawal transaction insert successfully");
            t
        },
        Err(e) => {
            println!("insert transaction failed {}", e);
            return Err(Status::InternalServerError)
        }
    };

    // send payment to lightning node
    match make_payment(&send_payment.payment_request).await {
        Some(_) => (),
        None => return Err(Status::InternalServerError)
    }

    let new_state = "SETTLED";
    let updated_transaction = sqlx::query( "UPDATE lightningchess_transaction SET state=$1, amount=$2 WHERE transaction_id=$3")
        .bind(new_state)
        .bind(&withdrawal_amt_neg)
        .bind(withdrawal_transaction.transaction_id)
        .execute(&mut tx).await;

    match updated_transaction {
        Ok(_) => println!("successfully updated_transaction transaction id"),
        Err(e) => {
            println!("error updated_transaction transaction id : {}", e);
            return Err(Status::InternalServerError)
        }
    }

    let winner_balance = sqlx::query( "UPDATE lightningchess_balance set balance=balance + $1 WHERE username=$2")
        .bind(&withdrawal_amt_neg)
        .bind(&user.username)
        .execute(&mut tx).await;

    match winner_balance {
        Ok(_) => println!("successfully payed admin"),
        Err(e) => {
            println!("error paying admin admin_transaction transaction{}", e);
            return Err(Status::InternalServerError)
        }
    }

    // commit transaction
    let commit_result = tx.commit().await;
    return match commit_result {
        Ok(_) => {
            println!("successfully committed");
            let send_payment_response = SendPaymentResponse {
                complete: true
            };
            Ok(serde_json::to_string(&send_payment_response).unwrap())
        },
        Err(e) => {
            println!("error committing: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

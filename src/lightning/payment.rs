use std::env;
use reqwest::Client;
use serde_json::json;
use crate::models::{DecodedPayment};

pub async fn decode_payment(payment_request: &str) -> Option<DecodedPayment> {
    let macaroon = env::var("LND_MACAROON").unwrap();

    let response = Client::new()
        .get(format!("https://lightningchess.m.voltageapp.io:8080/v1/payreq/{}", payment_request))
        .header("Grpc-Metadata-macaroon", macaroon)
        .send().await;

    return match response {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());
            let text = res.text().await;
            match text {
                Ok(text) => {
                    println!("text: {}", text);
                    let json_response: DecodedPayment = serde_json::from_str(&text).unwrap();
                    println!("DecodedPayment: {}", serde_json::to_string(&json_response).unwrap());
                    Some(json_response)
                }
                Err(e) => {
                    println!("error in text() :\n{}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("error from lnd decode_payment\n{}", e);
            None
        }
    };
}

pub async fn make_payment(payment_request: &str) -> Option<bool> {
    let macaroon = env::var("LND_MACAROON").unwrap();

    let body = json!({
        "payment_request": payment_request,
        "timeout_seconds": 10,
        "max_parts": 3,
        "fee_limit_sat": 10
    });
    println!("body: {}", body.to_string());

    let res_result = Client::new()
        .post(format!("https://lightningchess.m.voltageapp.io:8080/v2/router/send"))
        .json(&body)
        .header("Grpc-Metadata-macaroon", macaroon)
        .send().await;

    match res_result {
        Ok(mut res) => {
            let mut still_chunky = true;
            while still_chunky {
                let chunk_result = res.chunk().await;
                match chunk_result {
                    Ok(maybe_chunk) => match maybe_chunk {
                        Some(chunk) => println!("Chunk: {:?}", chunk),
                        None => {
                            println!("No chonks");
                            still_chunky = false;
                        }
                    },
                    Err(e) => {
                        println!("No more chunks {}", e);
                        return None;
                    }
                }
            }
        },
        Err(e) => {
            println!("error in v2/router/send :\n{}", e);
            return None;
        }
    }
    Some(true)
}
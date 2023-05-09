use crate::models::{AddInvoiceResponse, LookupInvoiceResponse, Challenge};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::{env, str};
use sha2::{Digest, Sha256};

pub async fn add_hodl_invoice(challenge: &Challenge, preimage_bytes: Vec<u8>) -> Option<AddInvoiceResponse> {
    let macaroon = env::var("LND_MACAROON").unwrap();
    let sats_str = challenge.sats.unwrap().to_string();
    let memo = format!("lightningchess.io chess game");
    let preimage_hash_bytes = Sha256::digest(preimage_bytes);
    let preimage_hash_base64 = base64::encode(preimage_hash_bytes);
    let body = json!({
        "hash": preimage_hash_base64,
        "value": sats_str,
        "memo": memo,
        "expiry": "1800"
    });
    println!("body: {}", body.to_string());

    let response = Client::new()
        .post("https://lightningchess.m.voltageapp.io:8080/v2/invoices/hodl")
        .json(&body)
        .header("Grpc-Metadata-macaroon", macaroon)
        .send().await;

    match response {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());
            let text = res.text().await;
            match text {
                Ok(text) => {
                    println!("text: {}", text);
                    let json_response: AddInvoiceResponse = serde_json::from_str(&text).unwrap();
                    println!("addHoldInvoiceResponse: {}", serde_json::to_string(&json_response).unwrap());
                    Some(json_response)
                }
                Err(e) => {
                    println!("error in text() :\n{}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("error from lnd addHoldInvoice\n{}", e);
            None
        }
    }
}

pub async fn lookup_hodl_invoice(payment_addr: &str) -> Option<LookupInvoiceResponse> {
    let macaroon = env::var("LND_MACAROON").unwrap();
    println!("payment_addr: {}", payment_addr);
    let base64_decoded_bytes = base64::decode(payment_addr).unwrap();
    let base64_url_safe_encoded = base64::encode_config(base64_decoded_bytes, base64::URL_SAFE);
    println!("base64_url_safe_encoded: {}", base64_url_safe_encoded);
    let response = Client::new()
        .get(format!("https://lightningchess.m.voltageapp.io:8080/v2/invoices/lookup?payment_addr={}", base64_url_safe_encoded))
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
                    let json_response: LookupInvoiceResponse = serde_json::from_str(&text).unwrap();
                    println!("lookupInvoiceResponse: {}", serde_json::to_string(&json_response).unwrap());
                    Some(json_response)
                }
                Err(e) => {
                    println!("error in text() :\n{}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("error from lnd lookup_invoice\n{}", e);
            None
        }
    };
}

pub async fn settle_hodl_invoice(preimage: &str) -> bool {
    let macaroon = env::var("LND_MACAROON").unwrap();
    let body = json!({
        "preimage": preimage
    });
    println!("preimage body: {}", body.to_string());
    let response = Client::new()
        .post("https://lightningchess.m.voltageapp.io:8080/v2/invoices/settle")
        .json(&body)
        .header("Grpc-Metadata-macaroon", macaroon)
        .send().await;

    return match response {
        Ok(res) => {

            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());
            StatusCode::from_u16(200).unwrap();
            res.status() == StatusCode::OK
        },
        Err(e) => {
            println!("error from lnd settle_invoice\n{}", e);
            false
        }
    }
}


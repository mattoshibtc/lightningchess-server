use std::env;
use reqwest::Client;
use serde_json::json;
use crate::models::{AddInvoiceResponse};

pub async fn add_invoice(sats: i64, memo: &str, preimage_bytes: Vec<u8>) -> Option<AddInvoiceResponse> {
    let macaroon = env::var("LND_MACAROON").unwrap();
    let sats_str = sats.to_string();
    let preimage_hash_base64 = base64::encode(preimage_bytes);
    let body = json!({
        "r_preimage": preimage_hash_base64,
        "value": sats_str,
        "memo": memo,
        "expiry": "1800"
    });
    println!("body: {}", body.to_string());

    let response = Client::new()
        .post("https://lightningchess.m.voltageapp.io:8080/v1/invoices")
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
                    println!("add_invoice: {}", serde_json::to_string(&json_response).unwrap());
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
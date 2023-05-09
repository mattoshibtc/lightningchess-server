use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{FromRow};

#[derive(Serialize, Deserialize)]
pub struct Account {
    pub username: String
}

pub struct AppConfig {
    pub url: String,
    pub fe_url: String
}

pub struct EnvVariables {
    pub macaroon: String // hex encoded
}

fn default_string() -> String {
    "".to_string()
}
fn default_i32() -> i32 {
    0
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Challenge {
    #[serde(default = "default_i32")]
    pub id: i32,
    #[serde(default = "default_string")]
    pub username: String,
    pub time_limit: Option<i32>, // seconds
    pub opponent_time_limit: Option<i32>, // seconds
    pub increment: Option<i32>, // seconds
    pub color: Option<String>,
    pub sats: Option<i64>,
    pub opp_username: String,
    pub status: Option<String>,
    pub lichess_challenge_id: Option<String>,
    pub created_on: Option<NaiveDateTime>, // UTC
    pub expire_after: Option<i32>, // seconds
    pub challenger_token: Option<String>
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Transaction {
    #[serde(default = "default_i32")]
    pub transaction_id: i32,
    #[serde(default = "default_string")]
    pub username: String,
    pub ttype: String,
    pub detail: String,
    pub amount: i64,
    pub state: String,
    pub preimage: Option<String>, // base64 encoded
    pub payment_addr: Option<String>, // base64 encoded
    pub payment_request: Option<String>,
    pub payment_hash: Option<String>,
    pub lichess_challenge_id: Option<String>
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Balance {
    #[serde(default = "default_i32")]
    pub balance_id: i32,
    #[serde(default = "default_string")]
    pub username: String,
    pub balance: i64
}
#[derive(Serialize, Deserialize)]
pub struct ChallengeAcceptRequest {
    pub id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct LichessChallenge {
    pub rated: bool,
    pub clock: LichessChallengeClock,
    pub color: String,
    pub variant: String,
    pub rules: String,
}

#[derive(Serialize, Deserialize)]
pub struct LichessChallengeClock {
    pub limit: String,
    pub increment: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddInvoiceRequest {
    pub sats: i64,
}

#[derive(Serialize, Deserialize)]
pub struct SendPaymentRequest {
    pub payment_request: String
}

#[derive(Serialize, Deserialize)]
pub struct LichessChallengeResponse {
    pub challenge: Url
}

#[derive(Serialize, Deserialize)]
pub struct LichessAddTimeResponse {
    pub ok: bool
}

#[derive(Serialize, Deserialize)]
pub struct LichessAcceptChallengeResponse {
    pub ok: bool
}

#[derive(Serialize, Deserialize)]
pub struct LichessExportGameResponse {
    pub id: String,
    pub rated: bool,
    pub variant: String,
    pub speed: String,
    pub perf: String,
    pub status: String,
    pub winner: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct LichessChallengeAcceptResponse {
    pub ok: bool
}

#[derive(Serialize, Deserialize)]
pub struct LichessPerf {
    pub games: i64,
    pub rating: i64,
    pub rd: i64,
    pub prog: i64,
}

#[derive(Serialize, Deserialize)]
pub struct LichessPerfs {
    pub blitz: LichessPerf,
    pub bullet: LichessPerf,
    pub correspondence: LichessPerf,
    pub classical: LichessPerf,
    pub rapid: LichessPerf,
}

#[derive(Serialize, Deserialize)]
pub struct LichessUser {
    pub id: String,
    pub username: String,
    pub perfs: LichessPerfs,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "seenAt")]
    pub seen_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String
}

#[derive(Serialize, Deserialize)]
pub struct Url {
    pub id: String,
    pub url: String
}

#[derive(Clone)]
pub struct User {
    pub access_token: String,
    pub username: String
}

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub username: String
}

#[derive(Serialize, Deserialize)]
pub struct LookupInvoice {
    pub payment_addr: String
}

#[derive(Serialize, Deserialize)]
pub struct SendPaymentResponse {
    pub complete: bool
}

// LND
#[derive(Serialize, Deserialize)]
pub struct AddInvoiceResponse {
    pub payment_request: String,
    pub add_index: String,
    pub payment_addr: String
}

#[derive(Serialize, Deserialize)]
pub struct DecodedPayment {
    pub destination: String,
    pub payment_hash: String,
    pub num_satoshis: String,
    pub timestamp: String,
    pub expiry: String,
    pub description: String,
    pub description_hash: String,
    pub fallback_addr: String,
    pub cltv_expiry: String,
    pub payment_addr: String,
    pub num_msat: String
}

#[derive(Serialize, Deserialize)]
pub struct LookupInvoiceResponse {
    pub memo: String,
    pub value: String,
    pub settled: bool,
    pub creation_date: String,
    pub settle_date: String,
    pub payment_request: String,
    pub expiry: String,
    pub amt_paid_sat: String,
    pub state: String
}
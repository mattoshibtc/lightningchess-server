use std::{error, fmt};
use rocket::http::Status;
use sqlx::Error;

pub type LightningChessResult<T> = Result<T, Box<dyn error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct ParseRequestError {
    pub(crate) m: String
}

impl fmt::Display for ParseRequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParseRequestError: {}", &self.m)
    }
}

impl error::Error for ParseRequestError {}

pub fn sqlx_err_to_status(e: Error) -> Status {
    println!("db error: {}", e);
    Status::InternalServerError
}
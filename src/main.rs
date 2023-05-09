#[macro_use] extern crate rocket;

use crate::config::parse_config;
use crate::endpoints::callback::callback;
use crate::endpoints::challenge::{accept_challenge, create_challenge, lookup_challenge, challenges};
use crate::endpoints::money::{add_invoice_endpoint, balance, transactions, lookup_transaction, send_payment_endpoint};
use crate::endpoints::login::login;
use crate::endpoints::lichess::lichess_user_endpoint;
use crate::endpoints::profile::profile;
use crate::models::{AppConfig, User};
use moka::future::Cache;
use rocket::fairing::AdHoc;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::HashMap;
use std::env;
use rocket::response::Redirect;
use sqlx::postgres::PgPoolOptions;

pub mod errors;
pub mod guard;
pub mod models;
pub mod lichess;
pub mod lightning;
pub mod endpoints;
pub mod config;

#[get("/")]
async fn index(app_config: &State<AppConfig>) -> Template {
    let mut context = HashMap::new();
    context.insert("fe_url", app_config.fe_url.to_string());
    Template::render("index", &context)
}

#[get("/<any_str>", rank = 100)]
async fn index_catch_all(app_config: &State<AppConfig>, any_str: String) -> Template {
    println!("index catch all / {}", any_str);
    let mut context = HashMap::new();
    context.insert("fe_url", app_config.fe_url.to_string());
    Template::render("index", &context)
}

#[get("/api/<_any_str>", rank = 2)]
async fn api_catch_all(_any_str: String) -> Redirect {
    Redirect::to("/login")
}

#[launch]
async fn rocket() -> _ {

    let db_url = env::var("DB_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await.unwrap();

    let cache: Cache<String, User> = Cache::new(10_000);

    rocket::build()
        .attach(AdHoc::try_on_ignite("appConfig", parse_config))
        .manage(pool)
        .manage(cache)
        .mount("/", routes![
            index,
            index_catch_all,
            api_catch_all,
            login,
            callback,
            profile,
            create_challenge,
            accept_challenge,
            lookup_challenge,
            lichess_user_endpoint,
            challenges,
            add_invoice_endpoint,
            balance,
            transactions,
            lookup_transaction,
            send_payment_endpoint])
        .attach(Template::fairing())
}

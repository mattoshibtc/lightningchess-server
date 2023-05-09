use cookie::SameSite;
use cookie::time::Duration;
use reqwest::Client;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use serde_json::json;
use crate::models::{AppConfig, TokenResponse};

#[get("/callback?<code>")]
pub async fn callback(code: String, app_config: &State<AppConfig>, cookies: &CookieJar<'_>) -> Option<Redirect> {
    println!("in callback");
    let redirect_uri = format!("{}/callback", &app_config.url);
    let code_verifier: String = match cookies.get_private("codeVerifier") {
        Some(cookie) => {
            let cv = cookie.value().to_string();
            cookies.remove_private(cookie);
            cv
        }
        None => {
            println!("No code verifier found!");
            "".to_string()
        }
    };

    let body = json!({
        "grant_type": "authorization_code",
        "redirect_uri": redirect_uri,
        "client_id": "lightningchess",
        "code": code,
        "code_verifier": code_verifier
    });
    println!("body: {}", body.to_string());

    return match Client::new()
        .post("https://lichess.org/api/token")
        .json(&body)
        .send().await {
        Ok(res) => {
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());

            match res.text().await {
                Ok(text) => {
                    println!("text!: {}", text);
                    let token_response: TokenResponse = serde_json::from_str(&text).unwrap();
                    let cookie = Cookie::build("llchess_access_token", token_response.access_token)
                        .same_site(SameSite::None)
                        .secure(true)
                        .max_age(Duration::days(365))
                        .finish();
                    cookies.add(cookie);
                }
                Err(e) => {
                    println!("error:\n{}", e);
                }
            };
            Some(Redirect::to(format!("{}/dashboard", &app_config.url)))
        },
        Err(e) => {
            println!("error:\n{}", e);
            None
        },
    }
}
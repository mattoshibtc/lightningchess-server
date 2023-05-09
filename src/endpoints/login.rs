use cookie::SameSite;
use cookie::time::Duration;
use rand::{distributions::Alphanumeric, Rng};
use rocket::{State};
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use sha2::{Digest, Sha256};
use crate::AppConfig;

#[get("/login")]
pub fn login(app_config: &State<AppConfig>, cookies: &CookieJar<'_>) -> Redirect {
    let redirect_uri = format!("{}/callback", &app_config.url);

    // generate code verifier and challenge
    let rand: Vec<u8>  = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(128)
        .collect();
    let verifier = base64::encode_config(&rand, base64::URL_SAFE_NO_PAD);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::encode_config(&digest, base64::URL_SAFE_NO_PAD);

    // add verifier to private cookie
    let cookie = Cookie::build("codeVerifier", verifier)
        .same_site(SameSite::None)
        .secure(true)
        .max_age(Duration::minutes(10))
        .finish();
    cookies.add_private(cookie);

    Redirect::to(format!("https://lichess.org/oauth?\
       response_type=code&\
       client_id=lightningchess&\
       redirect_uri={redirect_uri}&\
       scope=preference:read%20challenge:write&\
       code_challenge_method=S256&\
       code_challenge={challenge}")
    )
}
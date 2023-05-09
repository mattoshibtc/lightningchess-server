use rocket::{Build, Rocket};
use crate::AppConfig;

pub async fn parse_config(rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
    let fe_url: String = match rocket.figment().extract_inner::<String>("fe_url") {
        Ok(value) => {
            info!("fe url: {value}");
            value
        },
        Err(e) => {
            info!("error: {e}");
            "".to_string()
        }
    };

    match rocket.figment().extract_inner("url") {
        Ok(value) => {
            info!("api host: {value}");
            Ok(rocket.manage(AppConfig { url: value, fe_url } ))
        },
        Err(e) => {
            info!("error: {e}");
            Err(rocket)
        }
    }
}
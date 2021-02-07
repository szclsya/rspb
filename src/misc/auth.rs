use crate::PasteState;

use actix_web::dev::ServiceRequest;
use actix_web::{web, Error};

use actix_web_httpauth::extractors::basic::{BasicAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use blake2::{Blake2b, Digest};
use log::warn;

pub async fn validator(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> Result<ServiceRequest, Error> {
    let admins = &req
        .app_data::<web::Data<PasteState>>()
        .unwrap()
        .config
        .admins;
    let password_hash = match credentials.password() {
        Some(p) => format!("{:x}", Blake2b::digest(p.as_bytes())).to_string(),
        None => {
            return Err(AuthenticationError::from(Config::default()).into());
        }
    };

    let username = credentials.user_id().to_string();
    match admins.get(&username) {
        Some(local_hash) => {
            if local_hash == &password_hash {
                return Ok(req);
            } else {
                warn!(
                    "{:?} attempt to access admin, but wrong password.",
                    req.connection_info().realip_remote_addr()
                );
                return Err(AuthenticationError::from(Config::default()).into());
            }
        }
        None => {
            return Err(AuthenticationError::from(Config::default()).into());
        }
    }
}

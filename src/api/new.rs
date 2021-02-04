use crate::api::{ApiError, Response};
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::prelude::*;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use log::{debug, info};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

const ID_LEN: usize = 6;
const KEY_LEN: usize = 10;

fn gen_random_chars(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}

#[derive(Serialize)]
struct Info {
    id: String,
    key: String,
    expire_time: Option<DateTime<Utc>>,
}

pub async fn post(
    data: web::Data<PasteState>,
    mut payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    // Parse expire time
    let expire_time = match req.headers().get("Expire-After") {
        Some(t) => {
            let minutes = t.to_str()?.to_string().parse::<i64>()?;
            if minutes > 0 {
                Some(Utc::now() + Duration::minutes(minutes))
            } else {
                None
            }
        }
        None => None,
    };

    // Get unused key
    let mut id = gen_random_chars(ID_LEN);
    while data.storage.inner.exists(&id)? {
        id = gen_random_chars(ID_LEN);
    }
    let key = gen_random_chars(KEY_LEN);

    let mut res: Response<Info> = Response {
        success: false,
        message: String::new(),
        info: None,
    };

    info!("NEW paste {:?} expire at {:?}.", id, expire_time);

    // iterate over multipart stream
    let mut file = data.storage.inner.new(&id, &key).await?;
    while let Ok(Some(mut field)) = payload.try_next().await {
        debug!(
            "Processing field with disposition {:?}",
            field.content_disposition()
        );
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            match file.write_all(&data).await {
                Ok(_res) => continue,
                Err(_err) => {
                    return Err(ApiError::Unknown(
                        "Connection error: upload interrupted.".to_string(),
                    ));
                }
            };
        }
    }

    data.storage.inner.update_size(&id).await?;

    // Check if it's an empty paste
    if data.storage.inner.size(&id)? == 0 {
        data.storage.inner.delete(&id).await?;
        return Err(ApiError::BadRequest("Cannot create paste with no content.".to_string()));
    }

    // Set expire time
    if let Some(t) = expire_time {
        data.storage.inner.set_expire_time(&id, &t)?;
    }

    if let Some(name) = req.headers().get("Name") {
        data.storage.inner.set_name(&id, name.to_str()?)?;
    }

    let info = Info {
        id,
        key,
        expire_time,
    };

    res.success = true;
    res.info = Some(info);
    Ok(HttpResponse::Ok().json(&res))
}

use crate::api::{read_field, ApiError, Response};
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use anyhow::Result;
use chrono::prelude::*;
use chrono::Duration;
use futures::TryStreamExt;
use log::{debug, info};
use rand::{thread_rng, Rng};
use serde::Serialize;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz123456";
const ID_LEN: usize = 6;
const KEY_LEN: usize = 10;

fn gen_random_chars(len: usize) -> String {
    let mut rng = thread_rng();

    let res: String = (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    res
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
    _req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    // Get unused key
    let mut id = gen_random_chars(ID_LEN);
    while data.storage.inner.exists(&id)? {
        id = gen_random_chars(ID_LEN);
    }
    let key = gen_random_chars(KEY_LEN);

    // Set up empty values to be filled in (potentially)
    let mut name: Option<String> = None;
    let mut expire_time: Option<DateTime<Utc>> = None;

    // iterate over multipart stream
    let mut file = data.storage.inner.new(&id, &key).await?;
    while let Ok(Some(mut field)) = payload.try_next().await {
        let disposition = match field.content_disposition() {
            Some(d) => d,
            None => {
                data.storage.inner.delete(&id).await?;
                return Err(ApiError::BadRequest(
                    "Bad form: No disposition.".to_string(),
                ));
            }
        };
        match disposition.get_name() {
            Some("content") | Some("c") => {
                read_field(&mut field, &mut file).await?;
            }
            Some("name") => {
                let mut buf: Vec<u8> = Vec::new();
                read_field(&mut field, &mut buf).await?;
                name = Some(String::from_utf8(buf)?);
            }
            Some("expire_after") => {
                let mut buf: Vec<u8> = Vec::new();
                read_field(&mut field, &mut buf).await?;
                let m = String::from_utf8(buf)?;
                let minutes = m.parse::<i64>()?;
                if minutes > 0 {
                    expire_time = Some(Utc::now() + Duration::minutes(minutes));
                } else {
                    data.storage.inner.delete(&id).await?;
                    return Err(ApiError::BadRequest("Bad expire time.".to_string()));
                }
            }
            _ => {
                data.storage.inner.delete(&id).await?;
                return Err(ApiError::BadRequest("Bad form".to_string()));
            }
        }
    }

    // Update size && Check if it's an empty paste
    data.storage.inner.update_size(&id).await?;
    let mut meta = data.storage.inner.get_meta(&id)?;
    if meta.size == 0 {
        data.storage.inner.delete(&id).await?;
        return Err(ApiError::BadRequest(
            "Cannot create paste with no content.".to_string(),
        ));
    }

    // Set expire time
    if let Some(t) = expire_time {
        meta.expire_time = Some(t);
    }

    // Set name
    if name.is_some() && name.as_ref().unwrap().len() < 8000 {
        meta.name = name;
    }

    // Write back meta
    data.storage.inner.set_meta(&id, &meta)?;

    // Success!
    info!("NEW paste {:?} expire at {:?}.", id, expire_time);
    let res: Response<Info> = Response {
        success: true,
        message: String::new(),
        info: Some(Info {
            id,
            key,
            expire_time,
        }),
    };

    Ok(HttpResponse::Ok().json(&res))
}

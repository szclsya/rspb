use crate::api::{read_field, ApiError, Response};
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::prelude::*;
use chrono::Duration;
use futures::TryStreamExt;

pub async fn put(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let mut response: Response<()> = Response {
        success: false,
        message: String::new(),
        info: None,
    };

    let key = match req.headers().get("Key") {
        Some(k) => k.to_str().unwrap(),
        None => {
            response.message = "Please provide key.".to_string();
            return Err(ApiError::BadRequest("Please provide key.".to_string()));
        }
    };

    let meta = data.storage.inner.get_meta(&id)?;
    if !meta.validate(&key) {
        return Err(ApiError::BadRequest("Invalid key for paste.".to_string()));
    }

    modify(data, id, payload, req).await
}

pub async fn modify(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    mut payload: Multipart,
    _req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let mut response: Response<()> = Response {
        success: false,
        message: String::new(),
        info: None,
    };
    if !data.storage.inner.exists(&id)? {
        return Err(ApiError::NotFound);
    }
    let mut meta = data.storage.inner.get_meta(&id)?;

    // Read multipart form
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
                let mut file = data.storage.inner.update(&id).await?;
                read_field(&mut field, &mut file).await?;
            }
            Some("name") => {
                let mut buf: Vec<u8> = Vec::new();
                read_field(&mut field, &mut buf).await?;
                meta.name = Some(String::from_utf8(buf)?);
            }
            Some("expire_after") => {
                let mut buf: Vec<u8> = Vec::new();
                read_field(&mut field, &mut buf).await?;
                let m = String::from_utf8(buf)?;
                let minutes = m.parse::<i64>()?;
                if minutes > 0 {
                    meta.expire_time = Some(Utc::now() + Duration::minutes(minutes));
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
    // Write back meta
    data.storage.inner.set_meta(&id, &meta)?;

    // We have a success if we manage to get here
    response.success = true;
    Ok(HttpResponse::Ok().json(response))
}

use crate::PasteState;
use crate::api::{Response, ApiError};

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::prelude::*;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use tokio::io::AsyncWriteExt;

pub async fn put(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    mut payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let mut response: Response<()> = Response {
        success: false,
        message: String::new(),
        info: None
    };

    let headers = req.headers();

    let key = match req.headers().get("Key") {
        Some(k) => k.to_str().unwrap(),
        None => {
            response.message = "Please provide key.".to_string();
            return Err(ApiError::BadRequest("Please provide key.".to_string()));
        }
    };

    if !data.storage.inner.exists(&id)? {
        return Err(ApiError::NotFound);
    }

    if !data.storage.inner.validate(&id, &key)? {
        return Err(ApiError::BadRequest("Invalid key for paste.".to_string()));
    }

    match headers.get("Update-Content") {
        Some(arg) => {
            if arg == "y" {
                // iterate over multipart stream
                let mut file = data.storage.inner.update(&id).await?;
                while let Ok(Some(mut field)) = payload.try_next().await {
                    while let Some(chunk) = field.next().await {
                        let data = chunk.unwrap();
                        match file.write_all(&data).await {
                            Ok(_res) => continue,
                            Err(_err) => {
                                return Err(ApiError::Unknown("Connection error: upload interrupted.".to_string()));
                            }
                        };
                    }
                }
            }
        }
        None => (),
    }

    // Parse expire time
    match req.headers().get("Expire-After") {
        Some(t) => {
            let s = t.to_str()?.to_string();
            let minutes = s.parse::<i64>()?;
            if minutes > 0 {
                let t = Utc::now() + Duration::minutes(minutes);
                data.storage.inner.set_expire_time(&id, &t)?;
            }
        },
        None => ()
    };

    // We have a success if we manage to get here
    response.success = true;
    Ok(HttpResponse::Ok().json(response))
}

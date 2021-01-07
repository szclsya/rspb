use crate::ise_on_err;
use crate::misc::decode_expire_time;
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::prelude::*;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

#[derive(Serialize)]
struct Response {
    success: bool,
    message: Option<String>,
}

pub async fn put(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    mut payload: Multipart,
    req: HttpRequest,
) -> impl Responder {
    let mut response = Response {
        success: false,
        message: None,
    };

    let headers = req.headers();

    let key = match req.headers().get("Key") {
        Some(k) => k.to_str().unwrap(),
        None => {
            response.message = Some("Please provide key.".to_string());
            return HttpResponse::BadRequest()
                .body(serde_json::to_string_pretty(&response).unwrap());
        }
    };

    if !ise_on_err!(data.storage.inner.validate(&id, &key)) {
        response.message = Some("Invalid key for paste.".to_string());
        return HttpResponse::BadRequest().body(serde_json::to_string_pretty(&response).unwrap());
    }

    match headers.get("Update-Content") {
        Some(arg) => {
            if arg == "y" {
                // iterate over multipart stream
                let mut file = ise_on_err!(data.storage.inner.update(&id).await);
                while let Ok(Some(mut field)) = payload.try_next().await {
                    while let Some(chunk) = field.next().await {
                        let data = chunk.unwrap();
                        match file.write_all(&data).await {
                            Ok(_res) => continue,
                            Err(_err) => {
                                response.message =
                                    Some("Connection error: upload interrupted.".to_string());
                                let json = serde_json::to_string_pretty(&response).unwrap();
                                return HttpResponse::InternalServerError().body(&json);
                            }
                        };
                    }
                }
            }
        }
        None => (),
    }

    match headers.get("Expire-Time") {
        Some(arg) => {
            let time = match decode_expire_time(arg.to_str().unwrap()) {
                Ok(t) => t,
                Err(_err) => {
                    response.message = Some("Invalid expire time.".to_string());
                    return HttpResponse::BadRequest()
                        .body(serde_json::to_string_pretty(&response).unwrap());
                }
            };
            let expire_time = Utc::now() + Duration::minutes(time as i64);
            ise_on_err!(data.storage.inner.set_expire_time(&id, &expire_time));
        }
        None => (),
    }

    match req.headers().get("Syntax-Highlight") {
        Some(s) => {
            match data
                .storage
                .inner
                .gen_syntax_highlight(&id, &s.to_str().unwrap())
                .await
            {
                Ok(_ok) => (),
                Err(_err) => {
                    response.message = Some("Syntax highlighting failed.".to_string());
                    return HttpResponse::InternalServerError()
                        .body(serde_json::to_string_pretty(&response).unwrap());
                },
            }
        }
        None => (),
    }

    // We have a success if we manage to get here
    response.success = true;
    HttpResponse::Ok().body(serde_json::to_string_pretty(&response).unwrap())
}

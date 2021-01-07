use crate::ise_on_err;
use crate::misc::decode_expire_time;
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::prelude::*;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use log::{debug, info};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

fn gen_paste_id() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(6).collect()
}

fn gen_key() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(10).collect()
}

#[derive(Serialize)]
struct Info {
    id: String,
    key: String,
    expire_time: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct Response {
    success: bool,
    message: Option<String>,
    info: Option<Info>,
}

pub async fn post(
    data: web::Data<PasteState>,
    mut payload: Multipart,
    req: HttpRequest,
) -> impl Responder {
    let expire_time = match req.headers().get("Expire-After") {
        Some(t) => match decode_expire_time(&t.to_str().unwrap()) {
            Ok(t) => Some(Utc::now() + Duration::minutes(t as i64)),
            Err(err) => {
                return HttpResponse::BadRequest().body(err.to_string());
            }
        },
        None => None,
    };

    // Get unused key
    let mut id = gen_paste_id();
    while ise_on_err!(data.storage.inner.exists(&id)) {
        id = gen_paste_id();
    }
    let key = gen_key();

    let mut res = Response {
        success: false,
        message: None,
        info: None,
    };

    info!("NEW paste {:?} expire at {:?}.", id, expire_time);

    // iterate over multipart stream
    let mut file = ise_on_err!(data.storage.inner.new(&id, &key).await);
    while let Ok(Some(mut field)) = payload.try_next().await {
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            match file.write_all(&data).await {
                Ok(_res) => continue,
                Err(_err) => {
                    res.message = Some("Connection error: upload interrupted.".to_string());
                    let json = serde_json::to_string_pretty(&res).unwrap();
                    return HttpResponse::InternalServerError().body(&json);
                }
            };
        }
    }
    // See if we need to highlight it
    match req.headers().get("Syntax-Highlight") {
        Some(s) => {
            debug!("Generating highlight for {}", &id);
            match data
                .storage
                .inner
                .gen_syntax_highlight(&id, &s.to_str().unwrap())
                .await
            {
                Ok(_ok) => (),
                Err(_err) => res.message = Some("Syntax highlighting failed.".to_string()),
            }
        }
        None => (),
    }

    // Set expire time
    if let Some(t) = expire_time {
        ise_on_err!(data.storage.inner.set_expire_time(&id, &t));
    }

    let info = Info {
        id,
        key,
        expire_time,
    };

    res.success = true;
    res.info = Some(info);
    return HttpResponse::Ok().body(serde_json::to_string_pretty(&res).unwrap());
}

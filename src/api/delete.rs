use crate::ise_on_err;
use crate::misc::parse_query_string;
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    success: bool,
    message: Option<String>,
}

pub async fn delete(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let mut response = Response {
        success: false,
        message: None,
    };

    let key = match req.headers().get("Key") {
        Some(k) => k.to_str().unwrap(),
        None => {
            response.message = Some("Please provide key.".to_string());
            return HttpResponse::BadRequest()
                .body(serde_json::to_string_pretty(&response).unwrap());
        }
    };

    if !ise_on_err!(data.storage.inner.validate(&id, &key)) {
        response.message = Some("Bad key.".to_string());
        return HttpResponse::BadRequest().body(serde_json::to_string_pretty(&response).unwrap());
    }

    match data.storage.inner.delete(&id).await {
        Ok(_ok) => {
            response.success = true;
            return HttpResponse::Ok().body(serde_json::to_string_pretty(&response).unwrap());
        }
        Err(err) => {
            response.message = Some(err.to_string());
            return HttpResponse::BadRequest()
                .body(serde_json::to_string_pretty(&response).unwrap());
        }
    };
}

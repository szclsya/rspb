use crate::misc::parse_query_string;
use crate::storage::Response;
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use anyhow::{format_err, Result};
use log::{debug, warn};
use regex::Regex;

use actix_web::web::BytesMut;
use futures::TryStreamExt;

pub async fn get(
    data: web::Data<PasteState>,
    info: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    // Make sure this is a valid paste id
    let id_uri_re = Regex::new(r"^/[a-zA-Z0-9]{6}(\.|\?|$)").unwrap();
    if !id_uri_re.is_match(req.uri().path()) {
        return HttpResponse::NotFound().body("404 Not Found");
    }

    let info = info.into_inner();
    let mut id = info.clone();
    id.truncate(6); // Only use first 6 elements
    debug!("GET paste with id {}.", &id);

    // Get query strings
    let queries_result = parse_query_string(req.query_string());
    let args = match queries_result {
        Ok(arg) => arg,
        Err(_err) => {
            return HttpResponse::BadRequest().body("Error: Malformed query string.");
        }
    };

    // Get highlighted paste
    if args.contains_key("syntax") && args.get("syntax").unwrap() == "on" {
        debug!("Syntax highlighting requested.");
        match data.storage.inner.get_highlighted_html(&id).await {
            Ok(content) => {
                return HttpResponse::Ok().body(content);
            }
            Err(_e) => {
                return HttpResponse::NotFound().body("Error: Highlighted paste not found.");
            }
        }
    }

    // Get paste content
    let content = data.storage.inner.get(&id).await;
    match content {
        Ok(content) => match content {
            Response::Content(vec) => {
                return HttpResponse::Ok().body(vec);
            }
            Response::Stream(stream) => {
                let s = stream.map_ok(BytesMut::freeze);
                return HttpResponse::Ok().streaming(s);
            }
        },
        Err(err) => {
            debug!("GET paste with id {} failed: {:?}", &info, err);
            return HttpResponse::Ok().body("Error: Paste not found.");
        }
    }
}

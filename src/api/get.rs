use crate::misc::parse_query_string;
use crate::storage::Response;
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
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

    let mut id = info.clone();
    id.truncate(6); // Only use first 6 elements
    debug!("GET paste with id {}.", &id);

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
            return HttpResponse::NotFound().body("Error: Paste not found.");
        }
    }
}

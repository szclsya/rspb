use crate::storage::Response;
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::debug;
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
        Ok(content) => {
            let mut meta = match data.storage.inner.get_meta(&id) {
                Ok(m) => m,
                Err(_e) => {
                    return HttpResponse::InternalServerError().body("Internal Server Error");
                }
            };

            // Get size
            let size = meta.size;
            let name = meta.name.clone().unwrap_or("".to_string());
            // Record atime
            match meta.atime {
                Some(t) => {
                    let now = chrono::Utc::now();
                    if now - t > chrono::Duration::minutes(60) {
                        meta.atime = Some(now);
                        // It's fine if it fails
                        let _ = data.storage.inner.set_meta(&id, &meta);
                    }
                }
                None => {
                    meta.atime = Some(chrono::Utc::now());
                    let _ = data.storage.inner.set_meta(&id, &meta);
                }
            }

            match content {
                Response::Content(vec) => {
                    return HttpResponse::Ok()
                        .header("Content-Length", size)
                        .header("Name", name)
                        .body(vec);
                }
                Response::Stream(stream) => {
                    let s = stream.map_ok(BytesMut::freeze);
                    return HttpResponse::Ok()
                        .header("Content-Length", size)
                        .header("Name", name)
                        .streaming(s);
                }
            }
        }
        Err(err) => {
            debug!("GET paste with id {} failed: {:?}", &info, err);
            return HttpResponse::NotFound().body("Error: Paste not found.");
        }
    }
}

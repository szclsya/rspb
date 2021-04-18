use crate::storage::Response;
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::debug;
use regex::Regex;
use yarte::Template;

#[derive(Template)]
#[template(path = "code")]
struct CodeTemplate {
    title: String,
    id: String,
    name: String,
    file_ext: String,
    file_content: String,
}

pub async fn render(
    data: web::Data<PasteState>,
    web::Path((id, lang)): web::Path<(String, String)>,
    req: HttpRequest,
) -> impl Responder {
    // Make sure this is a valid paste id
    let id_uri_re = Regex::new(r"^/[a-zA-Z0-9]{6}").unwrap();
    if !id_uri_re.is_match(req.uri().path()) {
        return HttpResponse::NotFound().body("404 Paste Not Found");
    }

    let mut id = id.clone();
    id.truncate(6); // Only use first 6 elements
    debug!("GET paste with id {} with lang {}.", &id, &lang);

    // Get paste content
    let content = data.storage.inner.get(&id).await;
    let code: String = match content {
        Ok(content) => match content {
            Response::Content(vec) => match String::from_utf8(vec) {
                Ok(s) => s,
                Err(_e) => "Bad content. Maybe not in UTF-8?".to_string(),
            },
            Response::Stream(_stream) => {
                return HttpResponse::PayloadTooLarge().body("The paste is too large to display.");
            }
        },
        Err(err) => {
            debug!("GET paste with id {} failed: {:?}", &id, err);
            return HttpResponse::NotFound().body("Error: Paste not found.");
        }
    };

    let name = match data.storage.inner.get_meta(&id) {
        Ok(meta) => match meta.name {
            Some(n) => n,
            None => "untitled".to_string(),
        },
        Err(_e) => return HttpResponse::InternalServerError().body("Internal Server Error"),
    };

    let ctx = CodeTemplate {
        title: data.config.site.name.clone(),
        name,
        id: id.clone(),
        file_ext: lang.clone(),
        file_content: code,
    };

    let html = ctx.call().unwrap();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(html)
}

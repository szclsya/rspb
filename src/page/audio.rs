use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::debug;
use regex::Regex;
use yarte::Template;

#[derive(Template)]
#[template(path = "audio")]
struct AudioTemplate {
    title: String,
    slogan: String,
    id: String,
    filename: String,
    extension: String
}

pub async fn render(
    data: web::Data<PasteState>,
    web::Path(id): web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    // Make sure this is a valid paste id
    let id_uri_re = Regex::new(r"^/[a-zA-Z0-9]{6}").unwrap();
    if !id_uri_re.is_match(req.uri().path()) {
        return HttpResponse::NotFound().body("404 Paste Not Found");
    }

    let mut id = id;
    id.truncate(6); // Only use first 6 elements
    debug!("GET audio paste with id {}.", &id);

    // Get paste name
    let res = data.storage.inner.exists(&id);
    if res.is_err() | !res.unwrap() {
        return HttpResponse::NotFound().body("404 Paste Not Found");
    }

    let name = match data.storage.inner.get_meta(&id) {
        Ok(meta) => match meta.name {
            Some(n) => n,
            None => "untitled".to_string(),
        },
        Err(_e) => return HttpResponse::InternalServerError().body("Internal Server Error"),
    };

    // Get filename extensiion
    let sections = name.split('.').collect::<Vec<&str>>();
    let extension = match sections.len() {
        0 => "mp3".to_string(), // Try the most universal file
        _ => sections[sections.len()-1].to_string(),
    };

    let ctx = AudioTemplate {
        title: data.config.site.name.clone(),
        id: id.clone(),
        slogan: data.config.site.slogan.clone(),
        filename: name,
        extension,
    };

    let html = ctx.call().unwrap();
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

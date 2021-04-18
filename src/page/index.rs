use crate::PasteState;

use actix_web::{web, HttpResponse, Responder};
use yarte::Template;

#[derive(Template)]
#[template(path = "index")]
struct IndexTemplate {
    title: String,
    slogan: String,
    description: String,
    url: String,
}

pub async fn render(data: web::Data<PasteState>) -> impl Responder {
    let ctx = IndexTemplate {
        title: data.config.site.name.clone(),
        slogan: data.config.site.slogan.clone(),
        description: data.config.site.description.clone(),
        url: data.config.site.url.clone(),
    };

    let content = ctx.call().unwrap();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(content)
}

use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use yarte::Template;

#[derive(Template)]
#[template(path = "form")]
struct IndexTemplate {
    title: String,
    slogan: String,
}

pub async fn render(data: web::Data<PasteState>) -> impl Responder {
    let ctx = IndexTemplate {
        title: data.config.site.name.clone(),
        slogan: data.config.site.slogan.clone(),
    };

    let content = ctx.call().unwrap();

    HttpResponse::Ok().body(content)
}

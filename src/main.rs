mod storage;
use crate::storage::StorageBox;
mod api;
pub mod misc;
mod page;

use actix_web::{guard, middleware, rt, web, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use async_std::path::PathBuf;
use clap::Arg;
use log::warn;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

// Static files
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[derive(Deserialize, Clone)]
struct Config {
    base_dir: String,
    redis_address: Option<String>,
    bind_address: String,
    admins: HashMap<String, String>,
    site: SiteConfig,
    max_size: Option<u64>,
}

#[derive(Deserialize, Clone)]
struct SiteConfig {
    name: String,
    slogan: String,
    description: String,
    url: String,
}

pub struct PasteState {
    storage: StorageBox,
    config: Config,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    color_backtrace::install();
    env_logger::init();
    //yarte::recompile::when_changed();

    let matches = clap::App::new("rspb")
        .version("0.1")
        .about("Really Simple Paste Bin")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Set a config file")
                .required(true),
        )
        .get_matches();

    let config_path = PathBuf::from(matches.value_of("config").unwrap());
    let config: Config = toml::from_str(&std::fs::read_to_string(&config_path)?)?;

    let base_dir = PathBuf::from(&config.base_dir);
    let storage = StorageBox::new(&base_dir, config.redis_address.clone())
        .await
        .expect("Failed to initialize storage backend");

    // Periodically check paste expire
    let s1 = storage.clone();
    let max_size = config.max_size.map(|size| size*1024*1024);
    rt::spawn(async move {
        let mut counter = 0;
        loop {
            rt::time::delay_for(Duration::from_secs(60)).await;
            if counter > 60 {
                match s1.inner.cleanup(max_size).await {
                    Ok(_ok) => (),
                    Err(err) => warn!("{}", &err.to_string()),
                }
                counter = 0;
            } else {
                match s1.inner.cleanup(None).await {
                    Ok(_ok) => (),
                    Err(err) => warn!("{}", &err.to_string()),
                }
                counter += 1;
            }
        }
    });

    // Run http server
    let c2 = config.clone();
    HttpServer::new(move || {
        let generated = generate();
        let auth = HttpAuthentication::basic(misc::auth::validator);
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .data(PasteState {
                storage: storage.clone(),
                config: c2.clone(),
            })
            .service(
                web::resource("/f").route(web::route().guard(guard::Get()).to(page::form::render)),
            )
            .service(
                web::scope("/admin")
                    .wrap(auth)
                    .service(
                        web::resource("/list")
                            .route(web::route().guard(guard::Get()).to(api::admin::list::get)),
                    )
                    .service(
                        web::resource("/{paste_id}")
                            .route(web::route().guard(guard::Put()).to(api::admin::paste::put))
                            .route(
                                web::route()
                                    .guard(guard::Delete())
                                    .to(api::admin::paste::delete),
                            ),
                    ),
            )
            .service(
                web::resource("/")
                    .route(web::route().guard(guard::Get()).to(page::index::render))
                    .route(web::route().guard(guard::Post()).to(api::new::post)),
            )
            .service(
                web::resource("/{paste_id}")
                    .route(web::route().guard(guard::Delete()).to(api::delete::delete))
                    .route(web::route().guard(guard::Get()).to(api::get::get))
                    .route(web::route().guard(guard::Head()).to(api::get::head))
                    .route(web::route().guard(guard::Put()).to(api::modify::put)),
            )
            .service(
                web::resource("/{paste_id}/audio")
                    .route(web::route().guard(guard::Get()).to(page::audio::render)),
            )
            .service(
                web::resource("/{paste_id}/{lang}")
                    .route(web::route().guard(guard::Get()).to(page::code::render)),
            )
            .service(actix_web_static_files::ResourceFiles::new(
                "/static", generated,
            ))
    })
    .bind(&config.bind_address)?
    .run()
    .await
}

mod storage;
use crate::storage::StorageBox;
mod api;
pub mod misc;
mod page;

use actix_web::{guard, middleware, rt, web, App, HttpServer};
use async_std::path::PathBuf;
use clap::Arg;
use log::{debug, warn};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Clone)]
struct Config {
    base_dir: String,
    redis_address: Option<String>,
    bind_address: String,
    site: SiteConfig,
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
    rt::spawn(async move {
        loop {
            rt::time::delay_for(Duration::from_secs(60)).await;
            match s1.inner.cleanup().await {
                Ok(_ok) => (),
                Err(err) => warn!("{}", &err.to_string()),
            }
        }
    });

    // Run http server
    let c2 = config.clone();
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(PasteState {
                storage: storage.clone(),
                config: c2.clone(),
            })
            .service(
                web::resource("/")
                    .route(web::route().guard(guard::Get()).to(page::index::render))
                    .route(web::route().guard(guard::Post()).to(api::new::post)),
            )
            .service(
                web::resource("/{paste_id}")
                    .route(web::route().guard(guard::Delete()).to(api::delete::delete))
                    .route(web::route().guard(guard::Get()).to(api::get::get))
                    .route(web::route().guard(guard::Put()).to(api::modify::put)),
            )
    })
    .bind(&config.bind_address)?
    .run()
    .await
}

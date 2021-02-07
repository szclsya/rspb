use crate::api::{ApiError, Response};
use crate::storage::PasteMeta;
use crate::PasteState;
use chrono::prelude::*;
use serde::Serialize;

use actix_web::{web, HttpRequest, HttpResponse};

#[derive(Serialize)]
struct PasteAdminMeta {
    id: String,
    create_time: DateTime<Utc>,
    expire_time: Option<DateTime<Utc>>,
    atime: Option<DateTime<Utc>>,
    name: Option<String>,
    size: u64,
}

impl From<(String, PasteMeta)> for PasteAdminMeta {
    fn from(i: (String, PasteMeta)) -> Self {
        PasteAdminMeta {
            id: i.0,
            create_time: i.1.create_time,
            expire_time: i.1.expire_time,
            atime: i.1.atime,
            name: i.1.name,
            size: i.1.size,
        }
    }
}

pub async fn get(data: web::Data<PasteState>, _req: HttpRequest) -> Result<HttpResponse, ApiError> {
    let metas = data.storage.inner.get_all_meta()?;
    let admin_metas: Vec<PasteAdminMeta> =
        metas.into_iter().map(|m| PasteAdminMeta::from(m)).collect();
    let res = Response {
        success: true,
        message: "".to_string(),
        info: Some(admin_metas),
    };

    Ok(HttpResponse::Ok().json(res))
}

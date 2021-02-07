use crate::api::ApiError;
use crate::PasteState;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};

pub async fn put(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    if !data.storage.inner.exists(&id)? {
        return Err(ApiError::NotFound);
    }

    crate::api::modify::modify(data, id, payload, req).await
}

pub async fn delete(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    if !data.storage.inner.exists(&id)? {
        return Err(ApiError::NotFound);
    }

    crate::api::delete::delete_api(data, id, req).await
}

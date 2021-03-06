use crate::api::{ApiError, Response};
use crate::PasteState;

use actix_web::{web, HttpRequest, HttpResponse};

pub async fn delete(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let key = match req.headers().get("Key") {
        Some(k) => k.to_str().unwrap(),
        None => {
            return Err(ApiError::BadRequest("No key provided".to_string()));
        }
    };

    if !data.storage.inner.exists(&id)? {
        return Err(ApiError::NotFound);
    }

    if !data.storage.inner.get_meta(&id)?.validate(&key) {
        return Err(ApiError::Forbidden);
    }

    delete_api(data, id, req).await
}

pub async fn delete_api(
    data: web::Data<PasteState>,
    id: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    data.storage.inner.delete(&id).await?;

    // Success!
    let response: Response<()> = Response {
        success: true,
        message: String::new(),
        info: None,
    };
    Ok(HttpResponse::Ok().json(response))
}

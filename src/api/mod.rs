pub mod admin;
pub mod delete;
pub mod get;
pub mod modify;
pub mod new;

use log::error;
use actix_web::{error::ResponseError, http::header::ToStrError, http::StatusCode, HttpResponse};
use serde::Serialize;
use std::{ num::ParseIntError, string::FromUtf8Error };

#[derive(Serialize)]
pub struct Response<I> {
    success: bool,
    message: String,
    info: Option<I>,
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound,
    Forbidden,
    Unknown(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::BadRequest(msg) => msg.to_string(),
            Self::NotFound => "Paste Not Found".to_string(),
            Self::Forbidden => "Forbidden: Bad Key".to_string(),
            Self::Unknown(msg) => msg.to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        error!("Error when processing API call: {}", err.to_string());
        ApiError::Unknown("Internal Server Error".to_string())
    }
}

impl From<ParseIntError> for ApiError {
    fn from(_err: ParseIntError) -> Self {
        ApiError::BadRequest("Bad Integer in Request".to_string())
    }
}

impl From<ToStrError> for ApiError {
    fn from(_err: ToStrError) -> Self {
        ApiError::BadRequest("Bad Header".to_string())
    }
}

impl From<FromUtf8Error> for ApiError {
    fn from(_err: FromUtf8Error) -> Self {
        ApiError::BadRequest("Bad UTF-8 in form.".to_string())
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_m) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Unknown(_m) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response: Response<()> = Response {
            success: false,
            message: self.to_string(),
            info: None,
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

use actix_multipart::Field;
use tokio::io::AsyncWriteExt;
use std::marker::Unpin;
use futures::StreamExt;
async fn read_field<T>(field: &mut Field, mut to: T) -> Result<(), ApiError>
where
    T: AsyncWriteExt + Unpin,
{
    let mut size = 0;
    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        size += data.len();
        match to.write_all(&data).await {
            Ok(_res) => continue,
            Err(_err) => {
                return Err(ApiError::Unknown(
                    "Connection error: upload interrupted.".to_string(),
                ));
            }
        };
    }

    // Don't allow empty field
    if size == 0 {
        return Err(ApiError::BadRequest("Bad form: Empty field".to_string()));
    }
    Ok(())
}

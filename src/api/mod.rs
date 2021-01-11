pub mod delete;
pub mod get;
pub mod modify;
pub mod new;

use actix_web::{HttpResponse, error::ResponseError, http::StatusCode, http::header::ToStrError};
use std::num::ParseIntError;
use serde::{Serialize};

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
    Unknown(String)
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
        match err {
            _ => ApiError::Unknown("Internal Server Error".to_string())
        }
    }
}

impl From<ParseIntError> for ApiError {
    fn from(err: ParseIntError) -> Self {
        match err {
            _ => ApiError::BadRequest("Bad Integer in Request".to_string())
        }
    }
}

impl From<ToStrError> for ApiError {
    fn from(err: ToStrError) -> Self {
        match err {
            _ => ApiError::BadRequest("Bad Header".to_string())
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_m) => StatusCode::BAD_REQUEST,
            Self::NotFound  => StatusCode::NOT_FOUND,
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

use std::io;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no route matches the requested path")]
    NotFound,

    #[error("failed to read request body: {0}")]
    ReadBody(#[from] axum::Error),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),

    #[error("failed to elevate privileges to write /etc/hosts")]
    Elevate,

    #[error("cannot bind {ip}: address is not assigned to an interface\n  add it with: {cmd}")]
    BindUnavailable { ip: String, cmd: String },

    #[error("cannot bind port {port}: privileged ports (below 1024) require root\n  run: sudo apiproxy serve")]
    PrivilegedPort { port: u16 },
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::ReadBody(_) => StatusCode::BAD_REQUEST,
            Error::Upstream(_) => StatusCode::BAD_GATEWAY,
            Error::Io(_)
            | Error::Json(_)
            | Error::Elevate
            | Error::BindUnavailable { .. }
            | Error::PrivilegedPort { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

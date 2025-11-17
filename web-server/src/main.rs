use axum::{
    body::{Body, Bytes},
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use base64::prelude::*;
use tokio::net::TcpListener;
use tracing::info;

const BIND_ADDR: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = Router::new()
        .route("/decompile", post(decompile))
        .layer(DefaultBodyLimit::disable()); 

    let listener = TcpListener::bind(BIND_ADDR).await?;
    info!("Topaz Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("there was an IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid base64 data recieved: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Base64(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        Response::builder()
            .status(self.status_code())
            .body(Body::from(format!("{self}")))
            .unwrap()
    }
}

async fn decompile(body: Bytes) -> Result<String, Error> {
    let mut bytecode = Vec::new();
    BASE64_STANDARD.decode_vec(body, &mut bytecode)?;
    let decompiled = luau_lifter::decompile_bytecode(&bytecode, 203);
    info!("Successfully decompiled bytecode.");
    Ok(decompiled)
}

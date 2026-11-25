use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap},
    response::Response,
    Router,
};
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::routes::{load_all, Route};

const MAX_BODY: usize = 64 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    routes: Arc<RwLock<Vec<Route>>>,
}

pub async fn run(config: &str, port: u16) -> Result<()> {
    let routes = Arc::new(RwLock::new(load_all(config)));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    tokio::spawn(watch(config.to_owned(), routes.clone()));

    let app = Router::new()
        .fallback(proxy)
        .with_state(AppState { client, routes });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("serving on http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn watch(config: String, routes: Arc<RwLock<Vec<Route>>>) {
    let mut last = SystemTime::UNIX_EPOCH;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Ok(modified) = std::fs::metadata(&config).and_then(|m| m.modified())
            && modified != last
        {
            last = modified;
            *routes.write().await = load_all(&config);
        }
    }
}

fn find_route<'a>(routes: &'a [Route], path: &str) -> Option<(&'a Route, String)> {
    let mut best: Option<(&Route, String, usize)> = None;
    for route in routes {
        let key = format!("/{}", route.key.trim_start_matches('/'));
        let remainder = match path.strip_prefix(&key) {
            Some("") => String::new(),
            Some(rest) if rest.starts_with('/') => rest.to_owned(),
            _ => continue,
        };
        if best.as_ref().is_none_or(|(_, _, len)| key.len() > *len) {
            best = Some((route, remainder, key.len()));
        }
    }
    best.map(|(route, remainder, _)| (route, remainder))
}

fn build_target(url: &str, remainder: &str, query: Option<&str>) -> String {
    let mut target = url.trim_end_matches('/').to_owned();
    target.push_str(remainder);
    if let Some(query) = query {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn strip_hop_by_hop(headers: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (name, value) in headers {
        if is_hop_by_hop(name.as_str())
            || name == header::HOST
            || name == header::CONTENT_LENGTH
        {
            continue;
        }
        out.insert(name.clone(), value.clone());
    }
    out
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

async fn proxy(State(state): State<AppState>, req: Request) -> Result<Response> {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().map(str::to_owned);

    let target = {
        let routes = state.routes.read().await;
        match find_route(&routes, &path) {
            Some((route, remainder)) => build_target(&route.url, &remainder, query.as_deref()),
            None => return Err(Error::NotFound),
        }
    };

    let headers = strip_hop_by_hop(req.headers());

    let body = axum::body::to_bytes(req.into_body(), MAX_BODY).await?;

    let resp = state
        .client
        .request(method, &target)
        .headers(headers)
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    let headers = strip_hop_by_hop(resp.headers());

    let mut response = Response::new(Body::from_stream(resp.bytes_stream()));
    *response.status_mut() = status;
    *response.headers_mut() = headers;

    Ok(response)
}

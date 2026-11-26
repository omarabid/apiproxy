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

use crate::config::{self, Config, Route};
use crate::error::{Error, Result};
use crate::hosts;

const MAX_BODY: usize = 64 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    routes: Arc<RwLock<Vec<Route>>>,
}

pub async fn run(path: &str) -> Result<()> {
    let cfg = load_or_create_config(path)?;
    hosts::apply(&cfg.hostname, &cfg.bind_ip)?;

    let routes = Arc::new(RwLock::new(cfg.routes.clone()));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    tokio::spawn(watch(path.to_owned(), routes.clone()));

    let app = Router::new()
        .fallback(proxy)
        .with_state(AppState { client, routes });

    let ip: std::net::IpAddr = cfg.bind_ip.parse().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    let addr = SocketAddr::from((ip, cfg.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrNotAvailable => {
            let cmd = if cfg!(target_os = "macos") {
                format!("sudo ifconfig lo0 alias {}", cfg.bind_ip)
            } else {
                format!("sudo ip addr add {} dev lo", cfg.bind_ip)
            };
            return Err(Error::BindUnavailable {
                ip: cfg.bind_ip.clone(),
                cmd,
            });
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let cmd = if cfg!(target_os = "linux") {
                format!(
                    "run: sudo setcap 'cap_net_bind_service=+ep' {}",
                    std::env::current_exe()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "/path/to/apiproxy".to_string())
                )
            } else {
                "run: sudo apiproxy serve".to_string()
            };
            return Err(Error::PrivilegedPort { port: cfg.port, cmd });
        }
        Err(e) => return Err(e.into()),
    };
    println!("serving on {}", cfg.origin());
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_or_create_config(path: &str) -> Result<Config> {
    if !std::path::Path::new(path).exists() {
        let default = Config::default();
        config::save(path, &default)?;
        return Ok(default);
    }
    config::load(path)
}

async fn watch(path: String, routes: Arc<RwLock<Vec<Route>>>) {
    let mut last = SystemTime::UNIX_EPOCH;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified())
            && modified != last
        {
            last = modified;
            if let Ok(cfg) = config::load(&path) {
                *routes.write().await = cfg.routes;
            }
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

//! Configuration and route storage.
//!
//! A single JSON file holds both proxy settings and the list of registered
//! routes. Loading a missing file yields the defaults; saving serializes the
//! whole [`Config`] back to disk.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hosts;

/// Proxy settings and the routes it serves, persisted as one JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Hostname clients use to reach the proxy, e.g. `myapi`.
    pub hostname: String,
    /// IP the listener binds to, e.g. `127.0.0.1`.
    pub bind_ip: String,
    /// TCP port to listen on. Port 80 is omitted from displayed origins.
    pub port: u16,
    /// Registered endpoints, matched by path prefix at request time.
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// A single endpoint, mapping a path key to an upstream URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Path prefix matched against incoming requests.
    pub key: String,
    /// Upstream URL requests under [`Route::key`] are proxied to.
    pub url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: "myapi".to_owned(),
            bind_ip: "127.0.0.1".to_owned(),
            port: 80,
            routes: Vec::new(),
        }
    }
}

impl Config {
    /// The base URL clients use, omitting `:80`.
    pub fn origin(&self) -> String {
        if self.port == 80 {
            format!("http://{}", self.hostname)
        } else {
            format!("http://{}:{}", self.hostname, self.port)
        }
    }
}

/// Loads the config at `path`, or returns defaults if the file is absent.
pub fn load(path: &str) -> Result<Config> {
    if !Path::new(path).exists() {
        return Ok(Config::default());
    }
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

/// Serializes `config` to `path` as pretty JSON.
pub fn save(path: &str, config: &Config) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// Prints the whole config as pretty JSON.
pub fn show(path: &str) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&load(path)?)?);
    Ok(())
}

/// Sets the hostname, then refreshes `/etc/hosts`.
pub fn set_hostname(path: &str, value: &str) -> Result<()> {
    mutate(path, |c| c.hostname = value.to_owned())
}

/// Sets the bind IP, then refreshes `/etc/hosts`.
pub fn set_bind_ip(path: &str, value: &str) -> Result<()> {
    mutate(path, |c| c.bind_ip = value.to_owned())
}

/// Sets the listen port.
pub fn set_port(path: &str, value: u16) -> Result<()> {
    mutate(path, |c| c.port = value)
}

/// Applies `f` to the loaded config, saves it, and refreshes `/etc/hosts`.
fn mutate(path: &str, f: impl FnOnce(&mut Config)) -> Result<()> {
    let mut cfg = load(path)?;
    f(&mut cfg);
    save(path, &cfg)?;
    hosts::apply(&cfg.hostname, &cfg.bind_ip)?;
    show(path)
}

/// Registers (or replaces) a route under `key`, then saves.
pub fn add(path: &str, url: &str, key: &str) -> Result<()> {
    let mut cfg = load(path)?;
    cfg.routes.retain(|r| r.key != key);
    cfg.routes.push(Route {
        key: key.to_owned(),
        url: url.to_owned(),
    });
    save(path, &cfg)?;
    println!("added {key} -> {url}");
    Ok(())
}

/// Removes the route with `key`, then saves.
pub fn remove(path: &str, key: &str) -> Result<()> {
    let mut cfg = load(path)?;
    let before = cfg.routes.len();
    cfg.routes.retain(|r| r.key != key);
    save(path, &cfg)?;
    if cfg.routes.len() < before {
        println!("removed {key}");
    } else {
        println!("no key {key}");
    }
    Ok(())
}

/// Prints each route as `origin/key -> url`.
pub fn list(path: &str) -> Result<()> {
    let cfg = load(path)?;
    let origin = cfg.origin();
    for r in &cfg.routes {
        println!("{origin}/{} -> {}", r.key, r.url);
    }
    Ok(())
}

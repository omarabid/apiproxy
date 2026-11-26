use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hosts;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub bind_ip: String,
    pub port: u16,
    #[serde(default)]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub key: String,
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
    pub fn origin(&self) -> String {
        if self.port == 80 {
            format!("http://{}", self.hostname)
        } else {
            format!("http://{}:{}", self.hostname, self.port)
        }
    }
}

pub fn load(path: &str) -> Result<Config> {
    if !Path::new(path).exists() {
        return Ok(Config::default());
    }
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn save(path: &str, config: &Config) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

pub fn show(path: &str) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&load(path)?)?);
    Ok(())
}

pub fn set_hostname(path: &str, value: &str) -> Result<()> {
    mutate(path, |c| c.hostname = value.to_owned())
}

pub fn set_bind_ip(path: &str, value: &str) -> Result<()> {
    mutate(path, |c| c.bind_ip = value.to_owned())
}

pub fn set_port(path: &str, value: u16) -> Result<()> {
    mutate(path, |c| c.port = value)
}

fn mutate(path: &str, f: impl FnOnce(&mut Config)) -> Result<()> {
    let mut cfg = load(path)?;
    f(&mut cfg);
    save(path, &cfg)?;
    hosts::apply(&cfg.hostname, &cfg.bind_ip)?;
    show(path)
}

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

pub fn list(path: &str) -> Result<()> {
    let cfg = load(path)?;
    let origin = cfg.origin();
    for r in &cfg.routes {
        println!("{origin}/{} -> {}", r.key, r.url);
    }
    Ok(())
}

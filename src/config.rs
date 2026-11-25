use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hosts;

pub const CONFIG_PATH: &str = "apiproxy.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub bind_ip: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: "myapi".to_owned(),
            bind_ip: "127.0.0.1".to_owned(),
            port: 80,
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

pub fn show() -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&load(CONFIG_PATH)?)?);
    Ok(())
}

pub fn set_hostname(value: &str) -> Result<()> {
    mutate(|c| c.hostname = value.to_owned())
}

pub fn set_bind_ip(value: &str) -> Result<()> {
    mutate(|c| c.bind_ip = value.to_owned())
}

pub fn set_port(value: u16) -> Result<()> {
    mutate(|c| c.port = value)
}

fn mutate(f: impl FnOnce(&mut Config)) -> Result<()> {
    let mut cfg = load(CONFIG_PATH)?;
    f(&mut cfg);
    save(CONFIG_PATH, &cfg)?;
    hosts::apply(&cfg.hostname, &cfg.bind_ip)?;
    show()
}

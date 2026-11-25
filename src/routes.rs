use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub key: String,
    pub url: String,
}

pub fn load_all(path: &str) -> Vec<Route> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    BufReader::new(file)
        .lines()
        .map_while(std::result::Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

fn save(path: &str, routes: &[Route]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    for r in routes {
        writeln!(f, "{}", serde_json::to_string(r)?)?;
    }
    Ok(())
}

pub fn add(path: &str, url: &str, key: &str) -> Result<()> {
    let mut routes = load_all(path);
    routes.retain(|r| r.key != key);
    routes.push(Route {
        key: key.to_owned(),
        url: url.to_owned(),
    });
    save(path, &routes)?;
    println!("added {key} -> {url}");
    Ok(())
}

pub fn remove(path: &str, key: &str) -> Result<()> {
    let mut routes = load_all(path);
    let before = routes.len();
    routes.retain(|r| r.key != key);
    save(path, &routes)?;
    if routes.len() < before {
        println!("removed {key}");
    } else {
        println!("no key {key}");
    }
    Ok(())
}

pub fn list(path: &str, port: u16) -> Result<()> {
    for r in load_all(path) {
        println!("http://localhost:{port}/{} -> {}", r.key, r.url);
    }
    Ok(())
}

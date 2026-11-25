use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub key: String,
    pub url: String,
}

fn load(path: &str) -> Vec<Route> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

fn save(path: &str, routes: &[Route]) {
    let mut f = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .unwrap();
    for r in routes {
        writeln!(f, "{}", serde_json::to_string(r).unwrap()).unwrap();
    }
}

pub fn add(path: &str, url: &str, key: &str) {
    let mut routes = load(path);
    routes.retain(|r| r.key != key);
    routes.push(Route {
        key: key.to_string(),
        url: url.to_string(),
    });
    save(path, &routes);
    println!("added {key} -> {url}");
}

pub fn remove(path: &str, key: &str) {
    let mut routes = load(path);
    let before = routes.len();
    routes.retain(|r| r.key != key);
    save(path, &routes);
    if routes.len() < before {
        println!("removed {key}");
    } else {
        println!("no key {key}");
    }
}

pub fn list(path: &str, port: u16) {
    for r in load(path) {
        println!("http://localhost:{port}/{} -> {}", r.key, r.url);
    }
}

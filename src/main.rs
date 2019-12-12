use std::env;

use alfred::{Item, ItemBuilder};
use anyhow::Result;
use fst::automaton::Subsequence;
use regex::Regex;
use reqwest;
use rustdoc_seeker::RustDoc;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sled;
use sled::Db;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const SEARCH_URL: &str = "https://crates.io/api/v1/crates?q=";
const DOCS_RS_DOMAIN: &str = "https://docs.rs";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    eprintln!("args: {:?}", &args);

    let clear = match args.get(2) {
        Some(a) if a == "-f" => true,
        _ => false,
    };
    let alfred_arg = match args
        .get(1)
        .map(|arg| arg.split_whitespace().collect::<Vec<_>>())
    {
        None => return Ok(()),
        Some(arg) => arg,
    };
    let cache_dir = match alfred::env::workflow_cache() {
        Some(dir) => dir.join("a"),
        None => PathBuf::from_str("/tmp/docrs")?,
    };
    eprintln!("cache_dir: {:?}", &cache_dir);
    let crate_name = alfred_arg.get(0).map_or("", |s| *s);
    let symbol_name = alfred_arg.get(1).map_or("", |s| *s);
    let docs_rs = DocsRs::new(cache_dir);
    if clear {
        docs_rs.clear()?;
        eprintln!("clear db");
        return Ok(());
    }
    eprintln!("crate_name: {}, symbol_name: {}", crate_name, symbol_name);
    docs_rs.suggest(crate_name, symbol_name)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResp {
    crates: Vec<Crate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Crate {
    id: String,
    name: String,
    updated_at: String,
    created_at: String,
    downloads: i64,
    recent_downloads: i64,
    max_version: String,
    description: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    repository: Option<String>,
    exact_match: bool,
}

impl Crate {
    fn url(&self) -> String {
        build_crate_url(&self.name, &self.max_version)
    }
}

impl From<Crate> for Item<'static> {
    fn from(c: Crate) -> Self {
        let name = format!("{}-{}", &c.name, &c.max_version);
        let subtitle = format!(
            "[{}/{}]{}",
            c.recent_downloads,
            c.downloads,
            c.description.as_ref().unwrap_or(&"".to_string())
        );
        ItemBuilder::new(name)
            .subtitle(subtitle)
            .arg(c.url())
            .autocomplete(format!("{} ", c.name))
            .into_item()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Symbol {
    path: String,
    description: String,
    url: String,
}

impl From<Symbol> for Item<'static> {
    fn from(sym: Symbol) -> Self {
        ItemBuilder::new(sym.path)
            .subtitle(sym.description)
            .arg(sym.url)
            .into_item()
    }
}

struct DocsRs {
    db: Db,
}

impl DocsRs {
    fn new(cache_dir: impl AsRef<Path>) -> Self {
        let db = Db::open(cache_dir).unwrap();
        DocsRs { db }
    }

    fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }

    fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if let Ok(Some(value)) = self.db.get(key) {
            eprintln!("get from cache: {}", key);
            let crate_: T = serde_json::from_slice(&value)?;
            return Ok(Some(crate_));
        };
        Ok(None)
    }

    fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        eprintln!("set to cache: {}", key);
        let v = serde_json::to_vec(value)?;
        self.db.insert(key, v)?;
        Ok(())
    }

    fn search_crate(&self, crate_name: &str) -> Result<Vec<Crate>> {
        let key = format!("crate:{}", crate_name);
        if let Ok(Some(v)) = self.get::<Vec<Crate>>(&key) {
            return Ok(v);
        }
        let crates = search_crate(crate_name)?;
        self.set(&key, &crates)?;
        Ok(crates)
    }

    fn get_crate_search_index(&self, crate_name: &str, version: &str) -> Result<RustDoc> {
        let key = format!("crate-search-index:{}:{}", crate_name, version);
        if let Ok(Some(v)) = self.get::<RustDoc>(&key) {
            return Ok(v);
        }
        let search_index = get_crate_search_index(crate_name, version)?;
        self.set(&key, &search_index)?;
        Ok(search_index)
    }

    fn search_symbol(
        &self,
        crate_name: &str,
        version: &str,
        symbol_name: &str,
    ) -> Result<Vec<Symbol>> {
        let rustdoc = self.get_crate_search_index(crate_name, version)?;
        let seeker = rustdoc.build();

        let subsq = Subsequence::new(symbol_name);
        let mut items = vec![];
        let crate_url = build_crate_url(crate_name, version);
        for i in seeker.search(&subsq) {
            let mut path = String::new();
            i.fmt_naive(&mut path)?;
            let mut url = String::new();
            i.fmt_url(&mut url)?;
            items.push(Symbol {
                url: format!("{}/../{}", &crate_url, url),
                path,
                description: i.desc.to_string(),
            });
        }
        Ok(items)
    }

    fn suggest(&self, crate_name: &str, symbol: &str) -> Result<()> {
        let crates = self.search_crate(crate_name)?;
        let found_crate = crates.iter().find(|item| item.name == crate_name);

        if !symbol.is_empty() && found_crate.is_some() {
            if let Some(c) = found_crate {
                eprintln!("found crate: {:?}, search for symbol: {}", c, symbol);
                let symbols = self.search_symbol(&c.name, &c.max_version, symbol)?;
                let items: Vec<Item> = symbols.into_iter().map(|i| i.into()).collect();
                alfred::json::write_items(std::io::stdout(), &items)?;
            }
        } else {
            let items: Vec<Item> = crates.into_iter().map(|i| i.into()).collect();
            alfred::json::write_items(std::io::stdout(), &items)?;
        }

        Ok(())
    }
}

fn build_crate_url(crate_name: &str, version: &str) -> String {
    format!(
        "{}/{}/{}/{}",
        DOCS_RS_DOMAIN, &crate_name, &version, &crate_name
    )
}

fn search_crate(crate_name: &str) -> Result<Vec<Crate>> {
    eprintln!("search crate from net: {}", crate_name);
    let url = format!("{}{}", SEARCH_URL, crate_name);
    let resp: SearchResp = reqwest::blocking::get(&url)?
        .json()
        .expect("decode response");

    Ok(resp.crates)
}

fn get_crate_search_index(crate_name: &str, version: &str) -> Result<RustDoc> {
    eprintln!(
        "get crate symbols from net, crate: {}, version: {}",
        crate_name, version
    );

    let crate_url = build_crate_url(crate_name, version);
    let body: String = reqwest::blocking::get(crate_url.as_str())?.text()?;
    let regex =
        Regex::new(r#"<script[^</>]*?src="([^<>"]*?search-index[^</>"]*?)"[^</>]*?>"#).unwrap();
    let captures = regex.captures(&body).unwrap();
    let src = captures.get(1).unwrap().as_str();
    let js_url = format!("{}/{}", &crate_url, src);
    let data: String = reqwest::blocking::get(js_url.as_str())?.text()?;
    let rustdoc: RustDoc = data.parse()?;
    Ok(rustdoc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_search_crate() {
        let dir = TempDir::new("search_crate").unwrap();
        let docs_rs = DocsRs::new(dir.path());
        println!("{:#?}", docs_rs.search_crate("async-std").unwrap());
    }

    #[test]
    fn test_search_symbol() {
        let dir = TempDir::new("search_symbol").unwrap();
        let docs_rs = DocsRs::new(dir.path());
        println!(
            "{:#?}",
            docs_rs
                .search_symbol("async-std", "1.2.0", "spawn")
                .unwrap()
        );
    }

    #[test]
    fn test_suggest() {
        let dir = TempDir::new("suggest").unwrap();
        let docs_rs = DocsRs::new(dir.path());
        println!("{:#?}", docs_rs.suggest("async-std", "spawn").unwrap());
        println!("{:#?}", docs_rs.suggest("async-std", "").unwrap());
        println!("{:#?}", docs_rs.suggest("async-", "").unwrap());
    }
}

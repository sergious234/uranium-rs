use std::env;

use reqwest::{header::HeaderMap, RequestBuilder};
use tokio::task;

use crate::mod_searcher::Method;

pub trait Req {
    fn get(&self, url: &str, method: Method, body: &str) -> RequestBuilder;
}

#[derive(Clone)]
pub struct RinthRequester {
    cliente: reqwest::Client,
    headers: HeaderMap,
}

impl RinthRequester {
    pub fn new() -> RinthRequester {
        let mut req = RinthRequester {
            cliente: reqwest::Client::new(),
            headers: HeaderMap::new(),
        };

        let (_, rinth_api_key) = env::vars()
            .find(|(v, _)| v == "RINTH_API_KEY")
            .unwrap_or_default();

        req.headers
            .insert("x-api-key", rinth_api_key.parse().unwrap());
        req.headers
            .insert("Content-Type", "application/json".parse().unwrap());
        req.headers
            .insert("Accept", "application/json".parse().unwrap());

        req
    }
    pub fn search_by_url(
        &self,
        url: &str,
    ) -> task::JoinHandle<Result<reqwest::Response, reqwest::Error>> {
        let url = url.to_owned();
        tokio::task::spawn(self.cliente.get(url).headers(self.headers.clone()).send())
    }
}

impl Req for RinthRequester {
    fn get(&self, url: &str, _method: Method, _body: &str) -> RequestBuilder {
        let url = url.to_owned();
        self.cliente.get(url).headers(self.headers.clone())
    }
}

#[derive(Clone)]
pub struct CurseRequester {
    cliente: reqwest::Client,
    headers: HeaderMap,
}

unsafe impl Send for CurseRequester {}

impl CurseRequester {
    pub fn new() -> CurseRequester {
        let mut req = CurseRequester {
            cliente: reqwest::Client::new(),
            headers: HeaderMap::new(),
        };

        let (_, curse_api_key) = env::vars()
            .find(|(v, _)| v == "CURSE_API_KEY")
            .unwrap_or_default();

        req.headers
            .insert("x-api-key", curse_api_key.parse().unwrap());
        req.headers
            .insert("Content-Type", "application/json".parse().unwrap());
        req.headers
            .insert("Accept", "application/json".parse().unwrap());

        req
    }
}

impl Req for CurseRequester {
    fn get(&self, url: &str, method: Method, _body: &str) -> RequestBuilder {
        let url = url.to_owned();

        match method {
            Method::GET => self.cliente.get(&url),
            Method::POST => self.cliente.post(&url),
        }
        .headers(self.headers.clone())
    }
}

impl Req for reqwest::Client {
    fn get(&self, url: &str, _method: Method, _body: &str) -> reqwest::RequestBuilder {
        let url = url.to_owned();
        self.get(url)
    }
}

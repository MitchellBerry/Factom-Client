#![allow(dead_code, non_camel_case_types)]
use std::collections::HashMap;
use http::header::HeaderValue;
use serde_json::{Value, json};
use hyper_tls::HttpsConnector;
use serde::{Serialize, Deserialize};
pub use hyper::rt::{self, Future, Stream};
use hyper::{Method, Request, Body, Client};

pub mod api;
mod tests;

const WALLET_URI: &str = "http://localhost:8088/v2";
const FACTOMD_URI: &str = "http://localhost:8089/v2";
const API_VERSION: u8 = 2;
const JSONRPC : &str = "2.0";
const ID: u32 = 0;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum Outcome{
    result(Value),
    error(HashMap<String, Value>)
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Response{
    pub jsonrpc: String,
    pub id: u32,
    #[serde(flatten)]
    pub result: Outcome
}

impl Response {
    pub fn success(self)-> bool {
        match self.result {
            Outcome::error(_) => false,
            Outcome::result(_) => true
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiRequest{
    jsonrpc: String,
    id: u32,
    method: String,
    params: HashMap<String, Value>
}

impl ApiRequest {
    fn method(method: &str)-> ApiRequest{
        ApiRequest{
            jsonrpc: JSONRPC.to_string(),
            id: ID,
            method: method.to_string(),
            params: HashMap::new()
        }
    }

    fn parameters(&mut self, params: HashMap<String, Value>)-> &mut Self{
        self.params = params;
        self
    }

    fn to_json(&self)-> String{
        serde_json::to_string(&self).expect("error parsing json")
    }

}

#[derive(Debug)]
pub enum FetchError {
    Http(hyper::Error),
    Json(serde_json::Error),
}

impl From<hyper::Error> for FetchError {
    fn from(err: hyper::Error) -> FetchError {
        FetchError::Http(err)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(err: serde_json::Error) -> FetchError {
        FetchError::Json(err)
    }
}
#[derive(Clone, Default)]
pub struct Factom{
    uri: &'static str,
    wallet_uri: &'static str 
}

impl Factom {
    pub fn new()->Factom{
        Factom {
            uri: FACTOMD_URI,
            wallet_uri: WALLET_URI
        }
    }

    pub fn from_host(host: &str)->Factom{
        Factom {
            uri: to_static_str(format!("http://{}:8088/v{}", host, API_VERSION)),
            wallet_uri: to_static_str(format!("http://{}:8089/v{}", host, API_VERSION)),
        }
    }

    pub fn from_https_host(host: &str)->Factom{
        Factom {
            uri: to_static_str(format!("https://{}:8088/v{}", host, API_VERSION)),
            wallet_uri: to_static_str(format!("https://{}:8089/v{}", host, API_VERSION)),
        }
    }

    fn call(self, method: &str, params: HashMap<String, Value>)
                        ->  impl Future<Item=Response, Error=FetchError> {
            let uri = self.uri;
            self.inner_api_call(method, params, uri)
    }

    fn walletd_call(self, method: &str, params: HashMap<String, Value>)
                        ->  impl Future<Item=Response, Error=FetchError>{
            let uri = self.wallet_uri;
            self.inner_api_call(method, params, uri)
    }

    fn inner_api_call(self, method: &str, params: HashMap<String, Value>, uri: &str)
                        ->  impl Future<Item=Response, Error=FetchError> {
        let json_str = ApiRequest::method(method)
                                    .parameters(params)
                                    .to_json();
        let mut req = Request::new(Body::from(json_str));
        *req.method_mut() = Method::POST;
        *req.uri_mut() = uri.parse().unwrap_or_else(|_| panic!("Unable to parse URI: {}", uri));
        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json")
            );

        // https connector
        let https = HttpsConnector::new(4).expect("TLS initialization failed");

        let client = Client::builder().build::<_, hyper::Body>(https);
        client
            .request(req)
            .and_then(|res| {res.into_body().concat2()})
            .from_err::<FetchError>()
            .and_then(|json| {
                                let output: Response = serde_json::from_slice(&json)?;
                                Ok(output)
                            })
    }
}

// Retrieves future synchronously, blocks until Result is returned
pub fn fetch<F, R, E>(fut: F)-> Result<R, E>
    where
        F: Send + 'static + Future<Item = R, Error = E>,
        R: Send + 'static,
        E: Send + 'static,
    {
        let mut runtime = tokio::runtime::Runtime::new().expect("Unable to create a tokio runtime");
        runtime.block_on(fut)
    }

fn to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}



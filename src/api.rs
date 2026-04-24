use std::{collections::HashMap, error::Error, time::Duration, fmt};
use crate::credentials::Credentials;
use regex::Regex;
use reqwest::{Client, ClientBuilder, header::{HeaderMap, HeaderValue}, StatusCode};
use lazy_static::lazy_static;

fn parse_numeric_header(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name).and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok())
}

fn encode_unicode_as_html_entities(input: &str) -> String {
    let mut result = String::new();

    for ch in input.chars() {
        let code = ch as u32;
        if code > 127 {
            result.push_str(&format!("&#{};", code));
        } else {
            result.push(ch);
        }
    }

    result
}

pub struct Session {
    credentials: Option<Credentials>,
    pins: HashMap<String, String>,
    client: Client,
}

#[derive(Debug)]
pub enum ApiError {
    RateLimit(Duration),
    RequestError(reqwest::Error),
    TimedOut,
    NotFound,
    ServerError,
}

impl Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::RateLimit(duration) => {
                write!(f, "Rate limited, resets in {} seconds", duration.as_secs())
            },
            Self::RequestError(err) => {
                write!(f, "Request error {}", err)
            },
            Self::TimedOut => {
                write!(f, "Request timed out")
            },
            Self::NotFound => {
                write!(f, "Requested entity does not exist")
            },
            Self::ServerError => {
                write!(f, "API server error")
            }
        }
    }
}

lazy_static! {
    static ref SUCCESS_RE: Regex = Regex::new(r"<SUCCESS>(.+)</SUCCESS>").unwrap();
    static ref ID_RE: Regex = Regex::new(r"/id=([0-9]+)").unwrap();
}

fn extract_success_value(text: &str) -> Option<&str> {
    SUCCESS_RE.captures(text).and_then(|v| v.get(1).map(|v| v.as_str()))
}

fn extract_dispatch_id(text: &str) -> Option<u64> {
    ID_RE.captures(text).and_then(|v| v.get(1).and_then(|v| v.as_str().parse().ok()))
}

impl Session {
    pub fn new(user_agent: String, credentials: Option<Credentials>) -> Result<Self, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", HeaderValue::from_str(&user_agent).unwrap());

        Ok(Self { 
            credentials,
            pins: HashMap::new(),
            client: ClientBuilder::new().timeout(Duration::from_secs(10)).default_headers(headers).build()?,
        })
    }

    fn make_auth_headers(&self, nation: &str, password: Option<String>) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(password) = password {
            headers.insert("X-Password", HeaderValue::from_str(&password).unwrap());
        }

        if let Some(credentials) = &self.credentials && let Some(token) = credentials.get(nation) {
            headers.insert("X-Autologin", HeaderValue::from_str(token).unwrap());
        }

        if let Some(pin) = self.pins.get(nation) {
            headers.insert("X-Pin", HeaderValue::from_str(pin).unwrap());
        }

        headers
    }

    const API_URL: &'static str = "https://www.nationstates.net/cgi-bin/api.cgi";

    pub async fn make_request(
        &mut self, params: Vec<(&str, &str)>, nation: &str, password: Option<String>
    ) -> Result<(String, Option<String>), ApiError> {
        let response = match self.client.post(Self::API_URL).headers(
            self.make_auth_headers(nation, password)
        ).form(&params).send().await {
            Ok(r) => r,
            Err(err) => {
                if err.is_timeout() { 
                    eprintln!("Warning: API request timed out");
                    return Err(ApiError::TimedOut);
                }

                eprintln!("Warning: API request returned error {}", err);
                return Err(ApiError::RequestError(err));
            }
        };

        let headers = response.headers();

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            eprintln!("Warning: Hit rate limit - 429 Too Many Requests");

            return Err(ApiError::RateLimit(
                match parse_numeric_header(headers, "retry-after") {
                    Some(v) => Duration::from_secs(v),
                    None => Duration::from_secs(30)
                }
            ));
        }

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ApiError::NotFound);
        }

        if response.status().is_server_error() {
            return Err(ApiError::ServerError);
        }

        if let Some(pin) = headers.get("x-pin").and_then(|v| v.to_str().ok()) {
            self.pins.insert(nation.to_string(), pin.to_string());
        }

        let autologin = headers.get("x-autologin").and_then(|v| v.to_str().ok().map(|v| v.to_owned()));

        match response.text().await {
            Ok(t) => Ok((t, autologin)),
            Err(e) => Err(ApiError::RequestError(e))
        }
    }

    pub async fn make_request_with_retry(
        &mut self, params: Vec<(&str, &str)>, nation: &str, password: Option<String>
    ) -> Result<(String, Option<String>), ApiError> {
        let mut retried: bool = false;

        loop {
            match self.make_request(params.clone(), nation, password.clone()).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if retried { 
                        return Err(err);
                    }

                    retried = true;

                    match err {
                        ApiError::RateLimit(duration) => tokio::time::sleep(duration).await,
                        ApiError::TimedOut => tokio::time::sleep(Duration::from_secs(20)).await,
                        _ => {
                            return Err(err);
                        }
                    }
                }
            }
        }
    }

    pub async fn get_autologin_token(
        &mut self, nation: &str, password: String
    ) -> Result<Option<String>, ApiError> {
        let (_, autologin) = self.make_request_with_retry(
            vec![("nation", nation), ("q", "ping")], nation, Some(password)
        ).await?;

        Ok(autologin)
    }

    pub async fn create_dispatch(
        &mut self, 
        nation: &str, 
        title: &str,
        category: u64,
        subcategory: u64,
        content: &str,
    ) -> Result<Option<u64>, ApiError> {
        let category = category.to_string();
        let subcategory = subcategory.to_string();
        let text = encode_unicode_as_html_entities(content);

        let (response, _) = self.make_request_with_retry(
            vec![
                ("nation", nation),
                ("title", title),
                ("text", &text),
                ("category", &category),
                ("subcategory", &subcategory),
                ("c", "dispatch"),
                ("dispatch", "add"),
                ("mode", "prepare")
            ], nation, None
        ).await?;

        let Some(token) = extract_success_value(&response) else {
            return Ok(None);
        };

        let (response, _) = self.make_request_with_retry(
            vec![
                ("nation", nation),
                ("title", title),
                ("text", &text),
                ("category", &category),
                ("subcategory", &subcategory),
                ("c", "dispatch"),
                ("dispatch", "add"),
                ("mode", "execute"),
                ("token", token)
            ], nation, None
        ).await?;

        Ok(extract_dispatch_id(&response))
    }

    pub async fn edit_dispatch(
        &mut self, 
        nation: &str, 
        title: &str,
        category: u64,
        subcategory: u64,
        dispatchid: u64,
        content: &str,
    ) -> Result<Option<u64>, ApiError> {
        let category = category.to_string();
        let subcategory = subcategory.to_string();
        let dispatchid = dispatchid.to_string();
        let text = encode_unicode_as_html_entities(content);

        let (response, _) = self.make_request_with_retry(
            vec![
                ("nation", nation),
                ("title", title),
                ("text", &text),
                ("category", &category),
                ("subcategory", &subcategory),
                ("dispatchid", &dispatchid),
                ("c", "dispatch"),
                ("dispatch", "edit"),
                ("mode", "prepare")
            ], nation, None
        ).await?;

        let Some(token) = extract_success_value(&response) else {
            return Ok(None);
        };

        let (response, _) = self.make_request_with_retry(
            vec![
                ("nation", nation),
                ("title", title),
                ("text", &text),
                ("category", &category),
                ("subcategory", &subcategory),
                ("dispatchid", &dispatchid),
                ("c", "dispatch"),
                ("dispatch", "edit"),
                ("mode", "execute"),
                ("token", token)
            ], nation, None
        ).await?;
        
        Ok(extract_dispatch_id(&response))
    }
}
use crate::model::TlsInfo;
use crate::rate_limit;
use crate::ScanOptions;
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use std::time::{Duration, Instant};

pub struct InitialStats {
    pub status_code: u16,
    pub response_time_ms: u64,
    pub server_header: Option<String>,
    pub page_size_bytes: usize,
    pub body_text: String,
    pub tls: Option<TlsInfo>,
}

pub fn run(url: &str, opts: &ScanOptions) -> Result<(HeaderMap, InitialStats)> {
    rate_limit::throttle(opts.delay_ms);

    let client = Client::builder()
        .timeout(Duration::from_secs(opts.timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("failed to initialize HTTP client")?;

    let start = Instant::now();
    let resp = client.get(url).send().context("HTTP connection failed")?;
    let elapsed = start.elapsed().as_millis() as u64;

    let status_code = resp.status().as_u16();
    let headers = resp.headers().clone();
    let server_header = headers
        .get(reqwest::header::SERVER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let is_https = url.starts_with("https://");
    let tls = if is_https {
        Some(TlsInfo {
            // reqwest does not expose the negotiated TLS session details here.
            protocol: None,
            cipher: None,
            issuer: None,
            expiry: None,
        })
    } else {
        None
    };

    // Read full body for content/page size & downstream inspection
    let body_bytes = resp.bytes().context("failed to read response body")?;
    let page_size_bytes = body_bytes.len();
    let body_text = String::from_utf8_lossy(&body_bytes).to_string();

    let stats = InitialStats {
        status_code,
        response_time_ms: elapsed,
        server_header,
        page_size_bytes,
        body_text,
        tls,
    };

    Ok((headers, stats))
}

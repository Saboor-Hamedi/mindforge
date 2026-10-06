//! # `webscan`
//!
//! Pure, self-contained HTTP security scanning library.
//!
//! **Networking choice:** This library uses blocking HTTP calls (`reqwest::blocking`).
//! It is blocking by design. The caller (e.g. `app`) is responsible for running
//! `scan()` off the UI thread.

mod cookies;
mod disclosure;
mod headers;
mod injection;
pub mod model;
mod outdated;
mod rate_limit;
mod stats;

pub use model::{Category, Finding, ScanResult, Severity, TlsInfo};

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub full: bool,        // enable disclosure + injection checks
    pub probe_forms: bool, // enable injection specifically (requires full)
    pub delay_ms: u64,     // throttle applied before EVERY outbound request
    pub timeout_secs: u64,
    pub note: Option<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            full: false,
            probe_forms: false,
            delay_ms: 200,
            timeout_secs: 10,
            note: None,
        }
    }
}

/// Blocking. Runs HTTP requests. Caller must run this off the UI thread.
///
/// Execution order:
/// 1. `stats::run` runs first. If it fails (DNS, TLS, timeout), `scan()` returns `Err` immediately
///    and no subsequent modules run.
/// 2. If `stats::run` succeeds, `headers::check`, `cookies::check`, and `outdated::check` run unconditionally.
/// 3. If `opts.full` is true, `disclosure::run` runs.
/// 4. If `opts.full && opts.probe_forms`, `injection::run` runs. `probe_forms` without `full` is a no-op.
pub fn scan(url: &str, opts: &ScanOptions) -> anyhow::Result<ScanResult> {
    // 1. Initial connection & baseline metrics (always first)
    let (resp_headers, stats) = stats::run(url, opts)?;

    let is_https = url.starts_with("https://");
    let mut findings = Vec::new();

    // 2. Passive checks on existing response
    findings.extend(headers::check(&resp_headers, is_https));
    findings.extend(cookies::check(&resp_headers, is_https));
    findings.extend(outdated::check(&resp_headers, &stats.body_text));

    // 3. Active probes if full mode is enabled
    if opts.full {
        findings.extend(disclosure::run(url, opts));

        if opts.probe_forms {
            findings.extend(injection::run(&stats.body_text, opts));
        }
    }

    Ok(ScanResult {
        url: url.to_string(),
        status_code: stats.status_code,
        response_time_ms: stats.response_time_ms,
        tls: stats.tls,
        server_header: stats.server_header,
        page_size_bytes: stats.page_size_bytes,
        note: opts.note.clone(),
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_options_defaults() {
        let opts = ScanOptions::default();
        assert!(!opts.full);
        assert!(!opts.probe_forms);
        assert_eq!(opts.delay_ms, 200);
        assert_eq!(opts.timeout_secs, 10);
        assert!(opts.note.is_none());
    }

    #[test]
    fn test_probe_forms_without_full_is_noop_in_options() {
        let opts = ScanOptions {
            full: false,
            probe_forms: true,
            delay_ms: 0,
            timeout_secs: 5,
            note: None,
        };
        assert!(!opts.full);
        assert!(opts.probe_forms);
    }
}

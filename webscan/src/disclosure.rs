use crate::model::{Category, Finding, Severity};
use crate::rate_limit;
use crate::ScanOptions;
use reqwest::blocking::Client;
use std::time::Duration;

pub fn run(base_url: &str, opts: &ScanOptions) -> Vec<Finding> {
    let mut findings = Vec::new();

    let client = match Client::builder()
        .timeout(Duration::from_secs(opts.timeout_secs.min(5)))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(_) => return findings,
    };

    let base = base_url.trim_end_matches('/');

    let probe_paths = [
        ("/.env", Severity::High, "Environment file (.env) accessible", "Database credentials and secret keys may be publicly readable."),
        ("/.git/HEAD", Severity::High, "Git repository exposed (.git/HEAD)", "Full source code and version history might be downloadable."),
        ("/robots.txt", Severity::Low, "robots.txt file present", "Crawling policy file found; verify disallow rules do not disclose sensitive administrative endpoints."),
        ("/.well-known/security.txt", Severity::Low, "security.txt discovered", "Security policy and vulnerability reporting contact information located."),
    ];

    for (path, severity, title, desc) in probe_paths {
        rate_limit::throttle(opts.delay_ms);

        let target = format!("{}{}", base, path);
        if let Ok(resp) = client.get(&target).send() {
            let status = resp.status().as_u16();
            if status == 200 {
                // Verify content isn't a custom 404 HTML page pretending to be 200
                let text = resp.text().unwrap_or_default();
                let is_valid = match path {
                    "/.env" => {
                        text.contains('=') && !text.contains("<!DOCTYPE") && !text.contains("<html")
                    }
                    "/.git/HEAD" => text.starts_with("ref:") || text.len() == 41,
                    _ => !text.is_empty(),
                };

                if is_valid {
                    findings.push(Finding {
                        category: Category::Disclosure,
                        severity,
                        title: format!("{}: {}", title, path),
                        description: desc.to_string(),
                    });
                }
            }
        }
    }

    findings
}

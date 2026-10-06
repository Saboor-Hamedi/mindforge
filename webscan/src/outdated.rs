use crate::model::{Category, Finding, Severity};
use reqwest::header::HeaderMap;

pub fn check(headers: &HeaderMap, body: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Server header leaking granular version numbers
    if let Some(val) = headers.get(reqwest::header::SERVER) {
        if let Ok(srv) = val.to_str() {
            let has_version = srv.chars().any(|c| c.is_ascii_digit());
            if has_version {
                findings.push(Finding {
                    category: Category::Outdated,
                    severity: Severity::Low,
                    title: format!("Server version banner leaked: '{}'", srv),
                    description: "Revealing exact server product versions assists attackers in identifying known CVEs.".into(),
                });
            }
        }
    }

    // 2. X-Powered-By / X-AspNet-Version leakage
    if let Some(val) = headers.get("x-powered-by") {
        if let Ok(pby) = val.to_str() {
            findings.push(Finding {
                category: Category::Outdated,
                severity: Severity::Low,
                title: format!("Technology leaked via X-Powered-By: '{}'", pby),
                description: "Backend runtime or framework signature exposed in response headers."
                    .into(),
            });
        }
    }

    if let Some(val) = headers.get("x-aspnet-version") {
        if let Ok(asp) = val.to_str() {
            findings.push(Finding {
                category: Category::Outdated,
                severity: Severity::Low,
                title: format!("ASP.NET version leaked: '{}'", asp),
                description: "Exact framework version exposed.".into(),
            });
        }
    }

    // 3. HTML meta generator tag fingerprinting
    // This marker is ASCII, so ASCII-only case folding preserves byte offsets
    // into `body`; Unicode lowercase expansion would make those offsets diverge.
    let lower_body = body.to_ascii_lowercase();
    if let Some(pos) = lower_body.find("<meta name=\"generator\" content=\"") {
        let after = &body[pos + 32..];
        if let Some(end) = after.find('"') {
            let generator = &after[..end];
            findings.push(Finding {
                category: Category::Outdated,
                severity: Severity::Low,
                title: format!("CMS / Framework generator disclosed: '{}'", generator),
                description: "HTML generator tag reveals website software and build tooling."
                    .into(),
            });
        }
    }

    findings
}

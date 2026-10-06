use crate::model::{Category, Finding, Severity};
use reqwest::header::HeaderMap;

pub fn check(headers: &HeaderMap, is_https: bool) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Strict-Transport-Security (HSTS)
    if is_https {
        if let Some(val) = headers.get("strict-transport-security") {
            if let Ok(v_str) = val.to_str() {
                if !v_str.contains("max-age") {
                    findings.push(Finding {
                        category: Category::Headers,
                        severity: Severity::Low,
                        title: "HSTS header missing max-age".into(),
                        description: format!(
                            "HSTS value '{}' lacks valid max-age directive",
                            v_str
                        ),
                    });
                }
            }
        } else {
            findings.push(Finding {
                category: Category::Headers,
                severity: Severity::Medium,
                title: "Strict-Transport-Security (HSTS) missing".into(),
                description: "HTTPS connection does not enforce HSTS, leaving it vulnerable to downgrade attacks.".into(),
            });
        }
    }

    // 2. Content-Security-Policy (CSP)
    if !headers.contains_key("content-security-policy") {
        findings.push(Finding {
            category: Category::Headers,
            severity: Severity::Medium,
            title: "Content-Security-Policy (CSP) missing".into(),
            description: "No Content-Security-Policy header defined. XSS and data injection risks are elevated.".into(),
        });
    }

    // 3. X-Frame-Options (Clickjacking)
    if !headers.contains_key("x-frame-options") && !headers.contains_key("content-security-policy")
    {
        findings.push(Finding {
            category: Category::Headers,
            severity: Severity::Medium,
            title: "X-Frame-Options missing".into(),
            description:
                "No anti-clickjacking protection found. Page may be framed by external sites."
                    .into(),
        });
    }

    // 4. X-Content-Type-Options
    if let Some(val) = headers.get("x-content-type-options") {
        if val.to_str().unwrap_or("").to_lowercase() != "nosniff" {
            findings.push(Finding {
                category: Category::Headers,
                severity: Severity::Low,
                title: "X-Content-Type-Options not set to 'nosniff'".into(),
                description: "MIME sniffing is not strictly prevented.".into(),
            });
        }
    } else {
        findings.push(Finding {
            category: Category::Headers,
            severity: Severity::Low,
            title: "X-Content-Type-Options missing".into(),
            description: "Missing 'nosniff' directive allows browsers to guess MIME types.".into(),
        });
    }

    // 5. Referrer-Policy
    if !headers.contains_key("referrer-policy") {
        findings.push(Finding {
            category: Category::Headers,
            severity: Severity::Low,
            title: "Referrer-Policy missing".into(),
            description:
                "Referrer information may leak sensitive path or query parameters to third parties."
                    .into(),
        });
    }

    // 6. Permissions-Policy
    if !headers.contains_key("permissions-policy") && !headers.contains_key("feature-policy") {
        findings.push(Finding {
            category: Category::Headers,
            severity: Severity::Low,
            title: "Permissions-Policy missing".into(),
            description:
                "Browser features (camera, microphone, geolocation) are not explicitly restricted."
                    .into(),
        });
    }

    findings
}

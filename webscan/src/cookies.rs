use crate::model::{Category, Finding, Severity};
use reqwest::header::HeaderMap;

pub fn check(headers: &HeaderMap, is_https: bool) -> Vec<Finding> {
    let mut findings = Vec::new();

    for val in headers.get_all(reqwest::header::SET_COOKIE) {
        let cookie_str = match val.to_str() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let parts: Vec<&str> = cookie_str.split(';').map(|p| p.trim()).collect();
        if parts.is_empty() {
            continue;
        }

        let cookie_name = parts[0].split('=').next().unwrap_or("unknown");
        let attributes: Vec<String> = parts
            .iter()
            .skip(1)
            .map(|part| part.to_ascii_lowercase())
            .collect();
        let has_flag = |name: &str| attributes.iter().any(|attribute| attribute.trim() == name);
        let has_attribute = |name: &str| {
            attributes.iter().any(|attribute| {
                attribute
                    .split_once('=')
                    .is_some_and(|(key, _)| key.trim() == name)
            })
        };
        let same_site_none = attributes.iter().any(|attribute| {
            attribute
                .split_once('=')
                .is_some_and(|(key, value)| key.trim() == "samesite" && value.trim() == "none")
        });

        // 1. HttpOnly flag check
        if !has_flag("httponly") {
            findings.push(Finding {
                category: Category::Cookies,
                severity: Severity::Medium,
                title: format!("Cookie '{}' missing HttpOnly flag", cookie_name),
                description: format!("Cookie '{}' can be accessed by client-side JavaScript, increasing XSS session theft risk.", cookie_name),
            });
        }

        // 2. Secure flag check
        if is_https && !has_flag("secure") {
            findings.push(Finding {
                category: Category::Cookies,
                severity: Severity::High,
                title: format!("Cookie '{}' missing Secure flag", cookie_name),
                description: format!("Cookie '{}' transmitted over HTTPS lacks the Secure flag and may leak over unencrypted HTTP.", cookie_name),
            });
        }

        // 3. SameSite flag check
        if !has_attribute("samesite") {
            findings.push(Finding {
                category: Category::Cookies,
                severity: Severity::Low,
                title: format!("Cookie '{}' missing SameSite attribute", cookie_name),
                description: format!("Cookie '{}' does not specify SameSite (Lax/Strict), increasing CSRF vulnerability.", cookie_name),
            });
        } else if same_site_none && !has_flag("secure") {
            findings.push(Finding {
                category: Category::Cookies,
                severity: Severity::High,
                title: format!("Cookie '{}' has SameSite=None without Secure", cookie_name),
                description: "SameSite=None must be paired with Secure flag to prevent cookie rejection and leakage.".into(),
            });
        }
    }

    findings
}

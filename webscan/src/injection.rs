use crate::model::{Category, Finding, Severity};
use crate::ScanOptions;

pub fn run(body: &str, _opts: &ScanOptions) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lower_body = body.to_lowercase();

    // Scan for HTML <form> tags
    let mut search_from = 0;
    while let Some(form_idx) = lower_body[search_from..].find("<form") {
        let abs_start = search_from + form_idx;
        let form_end = match lower_body[abs_start..].find("</form>") {
            Some(e) => abs_start + e + 7,
            None => abs_start + 500.min(lower_body.len() - abs_start),
        };
        let form_slice = &lower_body[abs_start..form_end];

        let is_post =
            form_slice.contains("method=\"post\"") || form_slice.contains("method='post'");
        if is_post {
            // Check for common anti-CSRF token fields
            let has_csrf = form_slice.contains("csrf")
                || form_slice.contains("token")
                || form_slice.contains("_token")
                || form_slice.contains("authenticity_token");

            if !has_csrf {
                findings.push(Finding {
                    category: Category::Injection,
                    severity: Severity::Medium,
                    title: "Form missing anti-CSRF token".into(),
                    description: "POST form without recognizable anti-CSRF token input detected."
                        .into(),
                });
                break; // Report once per page
            }
        }
        search_from = form_end;
    }

    findings
}

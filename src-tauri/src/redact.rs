//! Defence-in-depth redaction.
//!
//! Logs and error payloads are attacker-adjacent surfaces: a GitHub token in a
//! log line is a credential leak. Every string that leaves the process passes
//! through here.

use std::sync::OnceLock;

use regex::{Regex, RegexSet};

use std::sync::Arc;

struct Rule {
    pattern: Regex,
    replacement: &'static str,
}

pub struct Redactor {
    rules: Vec<Rule>,
    probes: RegexSet,
}

impl Redactor {
    fn new() -> Self {
        let definitions: &[(&str, &str)] = &[
            // Classic + fine-grained GitHub tokens.
            (r"gh[pousr]_[A-Za-z0-9]{20,}", "[REDACTED_GITHUB_TOKEN]"),
            (r"github_pat_[A-Za-z0-9_]{20,}", "[REDACTED_GITHUB_TOKEN]"),
            // OAuth artefacts in query strings or headers.
            (r"(?i)(bearer\s+)[A-Za-z0-9._\-]{8,}", "${1}[REDACTED]"),
            (
                r#"(?i)((?:access|refresh)_token=)[^&\s"]+"#,
                "${1}[REDACTED]",
            ),
            (r#"(?i)(device_code=)[^&\s"]+"#, "${1}[REDACTED]"),
            (r#"(?i)(client_secret=)[^&\s"]+"#, "${1}[REDACTED]"),
            (r"(?i)(x-hub-signature-256:\s*)\S+", "${1}[REDACTED]"),
        ];

        let rules = definitions
            .iter()
            .map(|(pattern, replacement)| Rule {
                pattern: Regex::new(pattern)
                    .expect("redaction patterns are compile-time constants"),
                replacement,
            })
            .collect();

        let probes = RegexSet::new(definitions.iter().map(|(pattern, _)| pattern))
            .expect("redaction patterns are compile-time constants");

        Self { rules, probes }
    }

    /// Returns `input` with every known secret shape replaced.
    #[must_use]
    pub fn apply(&self, input: &str) -> String {
        if !self.probes.is_match(input) {
            return input.to_owned();
        }

        let mut output = input.to_owned();
        for rule in &self.rules {
            if rule.pattern.is_match(&output) {
                output = rule
                    .pattern
                    .replace_all(&output, rule.replacement)
                    .into_owned();
            }
        }
        output
    }
}

static REDACTOR: OnceLock<Arc<Redactor>> = OnceLock::new();

/// Shared redactor instance — compiled regexes are expensive, build them once.
#[must_use]
pub fn shared() -> Arc<Redactor> {
    Arc::clone(REDACTOR.get_or_init(|| Arc::new(Redactor::new())))
}

/// Redacts a single string using the process-wide rule set.
#[must_use]
pub fn redact(input: &str) -> String {
    shared().apply(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_ordinary_text_untouched() {
        let text = "workflow_run completed for acme/api in 42s";
        assert_eq!(redact(text), text);
    }

    #[test]
    fn removes_classic_tokens() {
        let text = "Authorization: token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let out = redact(text);
        assert!(!out.contains("ghp_"));
        assert!(out.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn removes_bearer_values_but_keeps_the_scheme() {
        let out = redact("header bearer abcdef1234567890 trailing");
        assert!(out.contains("bearer [REDACTED]"));
        assert!(!out.contains("abcdef1234567890"));
    }

    #[test]
    fn removes_oauth_query_parameters() {
        let out = redact("https://x.test/cb?access_token=abc123&state=ok");
        assert!(!out.contains("abc123"));
        assert!(out.contains("state=ok"));
    }
}

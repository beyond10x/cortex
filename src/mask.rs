//! Masking credential-shaped text before it is stored as evidence or sent to a model. This is not
//! PII redaction, and it is not a guarantee: it removes the shapes listed here.

use std::sync::OnceLock;

use regex::Regex;

fn rules() -> &'static [(&'static str, Regex)] {
    static RULES: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    RULES.get_or_init(|| {
        [
            (
                "private-key",
                r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
            ),
            ("aws-access-key", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
            ("gitlab-token", r"\bglpat-[A-Za-z0-9_\-]{20,}"),
            ("github-token", r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})"),
            ("slack-token", r"\bxox[abprs]-[A-Za-z0-9\-]{10,}"),
            ("tavily-key", r"\btvly-[A-Za-z0-9_\-]{16,}"),
            ("api-key", r"\bsk-[A-Za-z0-9_\-]{20,}"),
            (
                "jwt",
                r"\beyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}",
            ),
            (
                "secret-assignment",
                r#"(?i)\b(password|passwd|secret|api[_-]?key|access[_-]?token|auth[_-]?token)(\s*[:=]\s*)["']?[^\s"']{6,}"#,
            ),
        ]
        .into_iter()
        .map(|(name, pattern)| (name, Regex::new(pattern).expect("a valid pattern")))
        .collect()
    })
}

/// `text` with every listed credential shape replaced by `[masked:<shape>]`, and how many were.
pub fn mask(text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0;
    for (name, rule) in rules() {
        let replaced = if *name == "secret-assignment" {
            rule.replace_all(&out, |c: &regex::Captures| {
                count += 1;
                format!("{}{}[masked:{name}]", &c[1], &c[2])
            })
        } else {
            rule.replace_all(&out, |_: &regex::Captures| {
                count += 1;
                format!("[masked:{name}]")
            })
        };
        out = replaced.into_owned();
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn credential_shapes_are_masked_and_prose_is_kept() {
        // Assembled at runtime so no credential-shaped literal sits in the source.
        let token = format!("glpat-{}", "a".repeat(24));
        let tavily = format!("tvly-{}", "B".repeat(20));
        let text =
            format!("Use {token} or {tavily}. password = hunter2hunter2. The release ships.");
        let (masked, count) = mask(&text);
        assert_eq!(count, 3);
        assert!(!masked.contains(&token));
        assert!(!masked.contains(&tavily));
        assert!(!masked.contains("hunter2"));
        assert!(masked.contains("password = [masked:secret-assignment]"));
        assert!(masked.contains("The release ships."));
    }
}

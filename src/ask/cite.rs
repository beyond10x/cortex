//! Citation markers in an answer: `[node:<id>]` and `[evidence:<id>]` become Markdown links from
//! the spec's `ask.link_templates` (`cortex.ask.LinkTemplates`), and their ids are collected.
//!
//! A marker counts only when the reader returned its id: the id occurs, as a whole word, in a
//! tool result the model was shown in this answer. The model's text is not trusted to cite: a
//! marker naming any other id is an unsupported reference, written `[unsupported node:<id>]`,
//! never linked, and collected apart.

use std::sync::OnceLock;

use cortex_model::ask as ma;
use regex::Regex;

/// An answer with its markers turned into links, and the ids it cites, each once, in the order
/// they first appear: the verified ones by kind, the unsupported ones as `<kind>:<id>`.
#[derive(Debug, PartialEq, Eq)]
pub struct Cited {
    pub answer: String,
    pub nodes: Vec<String>,
    pub evidence: Vec<String>,
    pub unsupported: Vec<String>,
}

fn marker() -> &'static Regex {
    static MARKER: OnceLock<Regex> = OnceLock::new();
    MARKER.get_or_init(|| Regex::new(r"\[(node|evidence):([^\[\]\s]+)\]").expect("a valid regex"))
}

/// Every byte but the unreserved ones of RFC 3986, percent-encoded.
fn encode(id: &str) -> String {
    id.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A character that continues a word around an id: `n-1` is not in `n-10` or `xn-1`.
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

/// Whether `id` occurs in `text` as a whole word.
pub fn returned(text: &str, id: &str) -> bool {
    !id.is_empty()
        && text.match_indices(id).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + id.len()..].chars().next();
            !before.is_some_and(word) && !after.is_some_and(word)
        })
}

/// Turns each marker of `text` whose id one of `results` holds ([`returned`]) into
/// `[<id>](<url>)`, the URL being its kind's template with `{id}` replaced by the percent-encoded
/// id; a marker whose kind has no template stays as written. A marker whose id no result holds is
/// written `[unsupported <kind>:<id>]`.
pub fn cite(text: &str, templates: Option<&ma::LinkTemplates>, results: &[String]) -> Cited {
    let mut nodes = Vec::new();
    let mut evidence = Vec::new();
    let mut unsupported = Vec::new();
    let answer = marker()
        .replace_all(text, |c: &regex::Captures<'_>| {
            let (kind, id) = (&c[1], c[2].to_string());
            if !results.iter().any(|r| returned(r, &id)) {
                let reference = format!("{kind}:{id}");
                if !unsupported.contains(&reference) {
                    unsupported.push(reference.clone());
                }
                return format!("[unsupported {reference}]");
            }
            let (ids, template) = match kind {
                "node" => (&mut nodes, templates.and_then(|t| t.node.as_deref())),
                _ => (&mut evidence, templates.and_then(|t| t.evidence.as_deref())),
            };
            if !ids.contains(&id) {
                ids.push(id.clone());
            }
            match template {
                Some(template) => format!("[{id}]({})", template.replace("{id}", &encode(&id))),
                None => c[0].to_string(),
            }
        })
        .into_owned();
    Cited {
        answer,
        nodes,
        evidence,
        unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_become_links_and_ids_are_collected_once() {
        let templates = ma::LinkTemplates {
            node: Some("https://b.example/n/{id}".into()),
            evidence: None,
        };
        let results = ["{\"nodes\":[\"n-2\",\"a/b\"],\"evidence\":[\"e/1\"]}".to_string()];
        let cited = cite(
            "A [node:n 1] and [node:n-2], [node:n-2] per [evidence:e/1].",
            Some(&templates),
            &results,
        );
        // `[node:n 1]` holds a space: not a marker.
        assert_eq!(
            cited.answer,
            "A [node:n 1] and [n-2](https://b.example/n/n-2), [n-2](https://b.example/n/n-2) \
             per [evidence:e/1]."
        );
        assert_eq!(cited.nodes, vec!["n-2"]);
        assert_eq!(cited.evidence, vec!["e/1"]);
        assert!(cited.unsupported.is_empty());
        let encoded = cite("[node:a/b]", Some(&templates), &results);
        assert_eq!(encoded.answer, "[a/b](https://b.example/n/a%2Fb)");
    }

    /// Finding 21 of the 0.7.2 review: the reader returned `n-1` only.
    #[test]
    fn an_id_no_result_holds_is_unsupported_and_not_linked() {
        let templates = ma::LinkTemplates {
            node: Some("https://b.example/n/{id}".into()),
            evidence: Some("https://b.example/e/{id}".into()),
        };
        let results = ["node n-1 is the Atlas project; see n-10.".to_string()];
        let cited = cite(
            "Atlas [node:n-2] [node:n-1] [node:n-2] [evidence:n-1] [node:n-10x] [node:-1].",
            Some(&templates),
            &results,
        );
        assert_eq!(
            cited.answer,
            "Atlas [unsupported node:n-2] [n-1](https://b.example/n/n-1) \
             [unsupported node:n-2] [n-1](https://b.example/e/n-1) [unsupported node:n-10x] \
             [unsupported node:-1]."
        );
        assert_eq!(cited.nodes, vec!["n-1"]);
        assert_eq!(cited.evidence, vec!["n-1"]);
        assert_eq!(cited.unsupported, vec!["node:n-2", "node:n-10x", "node:-1"]);
    }

    #[test]
    fn an_id_is_returned_only_as_a_whole_word() {
        assert!(returned("node n-1 is", "n-1"));
        assert!(returned("{\"id\":\"n-1\"}", "n-1"));
        assert!(returned("ends with n-1.", "n-1"));
        assert!(!returned("n-10", "n-1"));
        assert!(!returned("xn-1", "n-1"));
        assert!(!returned("n-1_b", "n-1"));
        assert!(!returned("", "n-1"));
        assert!(!returned("anything", ""));
    }
}

//! Citation markers in an answer: `[node:<id>]` and `[evidence:<id>]` become Markdown links from
//! the spec's `ask.link_templates` (`cortex.ask.LinkTemplates`), and their ids are collected.

use std::sync::OnceLock;

use cortex_model::ask as ma;
use regex::Regex;

/// An answer with its markers turned into links, and the ids it cites, each once, in the order
/// they first appear.
#[derive(Debug, PartialEq, Eq)]
pub struct Cited {
    pub answer: String,
    pub nodes: Vec<String>,
    pub evidence: Vec<String>,
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

/// Turns each marker of `text` into `[<id>](<url>)`, the URL being its kind's template with
/// `{id}` replaced by the percent-encoded id; a marker whose kind has no template stays as written.
pub fn cite(text: &str, templates: Option<&ma::LinkTemplates>) -> Cited {
    let mut nodes = Vec::new();
    let mut evidence = Vec::new();
    let answer = marker()
        .replace_all(text, |c: &regex::Captures<'_>| {
            let (kind, id) = (&c[1], c[2].to_string());
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
        let cited = cite(
            "A [node:n 1] and [node:n-2], [node:n-2] per [evidence:e/1].",
            Some(&templates),
        );
        // `[node:n 1]` holds a space: not a marker.
        assert_eq!(
            cited.answer,
            "A [node:n 1] and [n-2](https://b.example/n/n-2), [n-2](https://b.example/n/n-2) \
             per [evidence:e/1]."
        );
        assert_eq!(cited.nodes, vec!["n-2"]);
        assert_eq!(cited.evidence, vec!["e/1"]);
        let encoded = cite("[node:a/b]", Some(&templates));
        assert_eq!(encoded.answer, "[a/b](https://b.example/n/a%2Fb)");
    }
}

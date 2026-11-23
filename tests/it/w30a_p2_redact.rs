//! Adversary pass 2 on unit redact of wave 20261030a (`story:redaction-patterns-hold`, correction
//! 7193dd0e). The correction made the `Url` pattern admit only parentheses and brackets it opens
//! and closes, so a match now stops at a `(` it cannot close or a `)` it did not open. Release
//! 0.6.1 admitted every parenthesis in a link and trimmed only an unbalanced `)` at its end, so
//! these links were replaced whole. Now the rest of the link after such a parenthesis is left in
//! the text sent to the model, in clear.

use std::collections::BTreeMap;

use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;

fn redactor(classes: &[m::RedactionClass]) -> Redactor {
    let policy = m::RedactionPolicy {
        classes: classes.to_vec(),
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: None,
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

fn run(text: &str) -> (String, BTreeMap<String, usize>) {
    let r = redactor(&[m::RedactionClass::Url]);
    let mut counts = r.counts();
    let out = r.redact(text, &mut counts);
    (out, counts)
}

/// A link whose query holds a `(` it never closes (a search typed by a person): the whole link is
/// one link, and nothing of it is left in clear.
#[test]
fn a_link_with_an_unclosed_parenthesis_is_replaced_whole() {
    let (out, _) = run("See https://crm.example/search?q=Jane(Doe-4711 for the record.");
    assert_eq!(out, "See [Url-1] for the record.", "{out}");
}

/// A link with a `)` it opened none for in the middle of its path: the whole link is one link.
#[test]
fn a_link_with_a_stray_closing_parenthesis_inside_is_replaced_whole() {
    let (out, _) = run("Open https://files.example/share/a1)Jane_Doe_contract.pdf today.");
    assert_eq!(out, "Open [Url-1] today.", "{out}");
}

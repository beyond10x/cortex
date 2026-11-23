//! Review cases of unit `timing-flakes`: the `Email` class reads an address a provider cut before
//! its top-level domain at the end of a text, and not a name followed by `@` and a commit or a
//! version there, such as the key of a document read at a commit (`group/repo@<sha>`). Before,
//! such a key was shown to the model as `[Email-1]`. The keys are synthetic.

use std::collections::BTreeMap;

use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;

fn redactor() -> Redactor {
    let policy = m::RedactionPolicy {
        classes: vec![m::RedactionClass::Url, m::RedactionClass::Email],
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: None,
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

/// `text` through one batch of the `Url` and `Email` classes, and the counts it took.
fn run(text: &str) -> (String, BTreeMap<String, usize>) {
    let r = redactor();
    let mut counts = r.counts();
    let out = r.redact(text, &mut counts);
    (out, counts)
}

#[test]
fn a_key_naming_a_commit_or_a_version_at_the_end_of_a_text_is_no_email_address() {
    for text in [
        "Source: example-group/widget-repo@3f2a9c1d0b",
        "example-group/widget-repo@3f2a9c1d0b4e5f60718293a4b5c6d7e8f9012345",
        "Source: widget-repo@9e8d7c6",
        "Pinned to widget-tool@1.4.2",
        "Source: example-group/widget-repo@3f2a9c1 ",
    ] {
        let (out, counts) = run(text);
        assert_eq!(out, text, "{text}");
        assert_eq!(counts["Email"], 0, "{text} -> {out}");
    }
}

#[test]
fn an_address_cut_before_its_top_level_domain_is_still_an_email_address() {
    for text in [
        "Write to jane.doe@exam",
        "Write to jane.doe@exam…",
        "Write to jane.doe@exam ",
    ] {
        let (out, counts) = run(text);
        assert!(!out.contains("jane.doe"), "{text} -> {out}");
        assert_eq!(counts["Email"], 1, "{text} -> {out}");
    }
    let (out, counts) = run("Write to jane.doe@example.org or widget-repo@3f2a9c1d.");
    assert_eq!(out, "Write to [Email-1] or widget-repo@3f2a9c1d.");
    assert_eq!(counts["Email"], 1);
}

//! Adversary cases for issue #74 (`story:redaction-name-beside-marks`), driven from the rule the
//! unit wrote into the `RareName` comment of `spec/domains/instance.yaml`: a word glued to a
//! lower-case letter, directly or through underscores, is an identifier and skipped whole;
//! *otherwise* an underscore and a letter of a script without case bound a word, as a space does;
//! and the `refuse_if_left` check reads words the same way, so it finds nothing left of a word that
//! was kept.
//!
//! Each case runs the public `cortex_cli::redact` API the run uses (`reserve` every text of the
//! batch, then `replace`, then `left` on what the model would be shown). The names are synthetic.

use std::collections::BTreeSet;

use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;

const RARE: m::RedactionClass = m::RedactionClass::RareName;

fn policy(replace: bool, known: &[&str], refuse: bool) -> Redactor {
    let policy = m::RedactionPolicy {
        classes: if replace { vec![RARE] } else { Vec::new() },
        rules: Vec::new(),
        known_names: (!known.is_empty()).then(|| known.iter().map(|n| n.to_string()).collect()),
        rare_limit: None,
        refuse_if_left: refuse.then(|| vec![RARE]),
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

/// What the model is shown of each text of one batch, and what the gate finds left in it.
fn shown(r: &Redactor, texts: &[&str]) -> Vec<(String, BTreeSet<&'static str>)> {
    let mut batch = r.batch();
    for t in texts {
        batch.reserve(t);
    }
    let out: Vec<String> = texts.iter().map(|t| batch.replace(t)).collect();
    out.into_iter()
        .map(|s| {
            let left = batch.left(&s);
            (s, left)
        })
        .collect()
}

fn one(r: &Redactor, text: &str) -> (String, BTreeSet<&'static str>) {
    shown(r, &[text]).remove(0)
}

// ---------------------------------------------------------------------------------------------
// The unit's own contract: "otherwise an underscore ... bounds a word, as a space does".
// ---------------------------------------------------------------------------------------------

/// A name glued through an underscore to an upper-case letter or a digit (`CV_Rowan.pdf`,
/// `2024_Quill.pdf`) is not glued to a lower-case letter, so by the spec the underscore bounds it
/// and it is a word. `joined_before` skips the underscore and `joins` accepts the `V` and the `4`,
/// so it is neither replaced nor seen by the gate.
#[test]
fn a_name_after_an_upper_case_letter_or_digit_and_an_underscore_is_not_shown() {
    let r = policy(true, &[], true);
    for text in [
        "attached: CV_Rowan.pdf",
        "attached: 2024_Quill.pdf",
        "attached: NDA_Thorne-Ashby.docx",
    ] {
        let (shown, left) = one(&r, text);
        for name in ["Rowan", "Quill", "Thorne", "Ashby"] {
            assert!(
                !shown.contains(name),
                "{name} reaches the model from {text:?}, and the gate finds {left:?}: {shown}"
            );
        }
    }
}

/// The same texts with the replacement off: the gate must refuse each one.
#[test]
fn the_gate_finds_a_name_after_an_upper_case_letter_or_digit_and_an_underscore() {
    let r = policy(false, &[], true);
    for text in ["attached: CV_Rowan.pdf", "attached: 2024_Quill.pdf"] {
        let (shown, left) = one(&r, text);
        assert_eq!(left, BTreeSet::from(["RareName"]), "{shown}");
    }
}

/// The whole-or-nothing rule of #71, through the same path: `Jean-Luc` after `CV_` fails the
/// boundary, the scan retries one letter on and finds `Luc` after the hyphen standing alone, so
/// the model is shown `CV_Jean-[...]`: half the name.
#[test]
fn a_hyphenated_name_after_an_upper_case_letter_and_an_underscore_is_not_shown_in_part() {
    let r = policy(true, &[], false);
    let (shown, _) = one(&r, "attached: CV_Jean-Luc.pdf");
    assert!(
        !shown.contains("Jean"),
        "half a name reaches the model: {shown}"
    );
}

// ---------------------------------------------------------------------------------------------
// Gate and replacement read a word differently once a placeholder stands before it.
// ---------------------------------------------------------------------------------------------

/// `Rowan_Quill` (a file name, a user name): `Rowan` starts the text and is replaced; `Quill` is
/// glued through the underscore to the `n` of `Rowan` and skipped whole as an identifier. The
/// model is shown `[...]_Quill`, and the gate, reading that text, sees `Quill` after `]_` as a
/// standalone rare word: the replacement keeps a word the gate then refuses, which the spec says
/// cannot happen, and with the gate off the surname reaches the model.
#[test]
fn the_second_half_of_an_underscored_full_name_is_not_shown() {
    let r = policy(true, &[], true);
    let (shown, left) = one(&r, "Signed by Rowan_Quill on Monday.");
    assert!(
        !shown.contains("Quill"),
        "a surname reaches the model, and the gate finds {left:?}: {shown}"
    );
}

// ---------------------------------------------------------------------------------------------
// The `\p{Lo}` cut in `capitalised_word` also cuts Latin-script names.
// ---------------------------------------------------------------------------------------------

/// Some Latin orthographies write letters that are `\p{Lo}`: click letters (`ǀ ǁ ǂ ǃ`, Khoekhoe
/// and Juǀ'hoan names) and the glottal stop `ʔ` (Salishan names). The base read `Nǀaqriaq` as one
/// word and replaced it; the new regex stops at `ǀ`, leaving `N` (no lower-case letter, not a
/// candidate) and `aqriaq` (no capital): nothing is replaced and the gate sees nothing.
#[test]
fn a_latin_name_with_a_click_or_glottal_letter_is_not_shown() {
    let r = policy(true, &[], true);
    for (text, part) in [
        ("Thanks to Nǀaqriaq for the map.", "aqriaq"),
        ("Thanks to Xwiʔamet for the map.", "Xwi"),
    ] {
        let (shown, left) = one(&r, text);
        assert!(
            !shown.contains(part),
            "part of a name reaches the model, and the gate finds {left:?}: {shown}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Outside the story's scope (pre-existing): kept ignored, reported for their own stories.
// ---------------------------------------------------------------------------------------------

/// A known name is replaced "however often it is seen", but its pattern is `\b<name>\b`, and
/// there is no `\b` between a CJK letter or an underscore and a Latin letter. Seen twice it is
/// not rare either, so the model is shown it. The gate does not run known names at all.
#[test]
#[ignore = "adversary finding: out of scope (known-name boundary, not RareName); pre-existing"]
fn a_known_name_glued_to_cjk_text_or_an_underscore_is_not_shown() {
    let r = policy(true, &["Müller"], true);
    let texts = ["会议上Müller发言了。", "Danke an _Müller_ für die Hilfe."];
    for (shown, left) in shown(&r, &texts) {
        assert!(
            !shown.contains("Müller"),
            "a known name reaches the model, and the gate finds {left:?}: {shown}"
        );
    }
}

/// Author lists mark affiliations with superscript digits glued to the name (`Rowan¹`). `¹` is
/// numeric, so `joins` reads it as part of the word and the name is neither replaced nor checked.
#[test]
#[ignore = "adversary finding: out of scope (digit boundary, not underscore or CJK); pre-existing"]
fn a_name_with_a_footnote_digit_is_not_shown() {
    let r = policy(true, &[], true);
    let (shown, left) = one(&r, "Authors: Ada Rowan¹, Bo Quill²");
    for name in ["Rowan", "Quill"] {
        assert!(
            !shown.contains(name),
            "a name reaches the model, and the gate finds {left:?}: {shown}"
        );
    }
}

/// A file name with a lower-case prefix (`lebenslauf_Rowan_Quill.pdf`) is an identifier by the
/// spec and skipped whole, so the name is shown there while the same name in the prose beside it
/// is replaced: the placeholder no longer hides who it stands for.
#[test]
#[ignore = "adversary finding: spec-sanctioned (identifier through underscores); judgement"]
fn a_name_in_a_lower_case_prefixed_file_name_is_not_shown() {
    let r = policy(true, &[], true);
    let (shown, left) = one(
        &r,
        "Rowan sent lebenslauf_Rowan_Quill.pdf and re_Jean-Luc.txt",
    );
    for name in ["Rowan", "Quill", "Jean", "Luc"] {
        assert!(
            !shown.contains(name),
            "{name} reaches the model, and the gate finds {left:?}: {shown}"
        );
    }
}

/// A titlecase digraph (`ǅ`, `\p{Lt}`) is not `\p{Lu}`, so a name written with one never starts a
/// capitalised word.
#[test]
#[ignore = "adversary finding: out of scope (titlecase letters); pre-existing, rarely typed"]
fn a_name_starting_with_a_titlecase_digraph_is_not_shown() {
    let r = policy(true, &[], true);
    let (shown, left) = one(&r, "Thanks to ǅemila for the map.");
    assert!(
        !shown.contains("emila"),
        "a name reaches the model, and the gate finds {left:?}: {shown}"
    );
}

// ---------------------------------------------------------------------------------------------
// Probes that held: kept as regression cases.
// ---------------------------------------------------------------------------------------------

/// A name glued to text of other scripts without case, on either side and at either end, in
/// markdown emphasis, after a katakana long-vowel mark or decomposed kana, and in NFD.
#[test]
fn a_name_beside_other_uncased_scripts_and_marks_is_not_shown() {
    let r = policy(true, &[], true);
    for text in [
        "ミーティングでRowanさんが話した。",
        "メンバーRowan",
        "か\u{3099}Rowanさん",
        "회의에서Rowan님이 말했다.",
        "คุณRowanพูดว่า",
        "شكراRowanشكرا",
        "תודהRowanתודה",
        "धन्यवादRowanजी",
        "Rowan发言了",
        "会议上Rowan",
        "Thanks to __Rowan__ here.",
        "Thanks to *_Rowan_* here.",
        "Thanks to Ro\u{0308}wan and O'Rowan.",
    ] {
        let (shown, left) = one(&r, text);
        assert!(
            !shown.contains("owan"),
            "a name reaches the model from {text:?}, and the gate finds {left:?}: {shown}"
        );
    }
}

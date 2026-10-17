//! Adversary cases for issue #71 (`story:redaction-keeps-frequent-hyphenated-word`), driven from
//! the rule the unit wrote into the `RareName` comment of `spec/domains/instance.yaml`: a word,
//! hyphenated or not, is replaced whole or not at all, and the `refuse_if_left` check reads words
//! the same way, so it finds nothing left of a word that was kept.
//!
//! Each case runs the public `cortex_cli::redact` API the run uses (`reserve` every text of the
//! batch, then `replace` or `replace_prefix`, then `left` on what the model would be shown). The
//! names are synthetic.

use std::collections::BTreeSet;

use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;

const RARE: m::RedactionClass = m::RedactionClass::RareName;

fn redactor(known: &[&str], rare_limit: Option<i64>, refuse: bool) -> Redactor {
    let policy = m::RedactionPolicy {
        classes: vec![RARE],
        rules: Vec::new(),
        known_names: (!known.is_empty()).then(|| known.iter().map(|n| n.to_string()).collect()),
        rare_limit,
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

// ---------------------------------------------------------------------------------------------
// Leaks: a rare name the model is shown.
// ---------------------------------------------------------------------------------------------

/// A double-barrelled surname whose second half is a known name: the known half is replaced, the
/// rare first half must be too. It is never counted (the word overlaps the known name), so it is
/// rare, and it is shown.
#[test]
fn the_rare_half_of_a_name_hyphenated_to_a_known_name_is_not_shown() {
    let r = redactor(&["Müller"], None, false);
    let out = shown(&r, &["Frau Schmidt-Müller rief zurück."]);
    let (text, _) = &out[0];
    assert!(
        !text.contains("Schmidt"),
        "the rare half of a hyphenated surname reaches the model: {text}"
    );
}

/// Markdown, Slack and wiki italics: `_Jean-Luc_`. The underscore before the word makes the
/// whole word not standalone, and the fix now skips it whole, so `Luc`, which the base replaced,
/// is shown, and the gate, reading words the same way, does not see it either.
#[test]
fn a_hyphenated_name_in_underscore_italics_is_not_shown() {
    let r = redactor(&[], None, true);
    let out = shown(&r, &["Thanks to _Jean-Luc_ for the review."]);
    let (text, left) = &out[0];
    assert!(
        !text.contains("Luc"),
        "a hyphenated name in italics reaches the model, and the gate finds {left:?}: {text}"
    );
}

/// The same for a single-word name in italics: it is never a rare name, in the replacement or in
/// the gate.
#[test]
fn a_name_in_underscore_italics_is_not_shown() {
    let r = redactor(&[], None, true);
    let out = shown(&r, &["Thanks to _Rowan_ for the review."]);
    let (text, left) = &out[0];
    assert!(
        !text.contains("Rowan"),
        "a name in italics reaches the model, and the gate finds {left:?}: {text}"
    );
}

/// Chinese and Japanese text writes a Latin-script name with no space around it. The CJK letter
/// before it makes the whole word not standalone; the fix skips it whole, so `Luc`, which the
/// base replaced, is shown.
#[test]
fn a_hyphenated_latin_name_glued_to_cjk_text_is_not_shown() {
    let r = redactor(&[], None, true);
    let out = shown(&r, &["会议上Jean-Luc发言了。"]);
    let (text, left) = &out[0];
    assert!(
        !text.contains("Luc"),
        "a hyphenated name in CJK text reaches the model, and the gate finds {left:?}: {text}"
    );
}

/// The single-word form of the case above.
#[test]
fn a_latin_name_glued_to_cjk_text_is_not_shown() {
    let r = redactor(&[], None, true);
    let out = shown(&r, &["会议上Rowan发言了。"]);
    let (text, left) = &out[0];
    assert!(
        !text.contains("Rowan"),
        "a name in CJK text reaches the model, and the gate finds {left:?}: {text}"
    );
}

/// The three names above, with `RareName` only refused, not replaced: the gate finds each one left.
#[test]
fn the_gate_finds_a_name_beside_an_underscore_cjk_text_or_a_known_name() {
    let refuse_only = |known: &[&str]| {
        let policy = m::RedactionPolicy {
            classes: Vec::new(),
            rules: Vec::new(),
            known_names: (!known.is_empty()).then(|| known.iter().map(|n| n.to_string()).collect()),
            rare_limit: None,
            refuse_if_left: Some(vec![RARE]),
        };
        compile(Some(&policy))
            .expect("a valid policy")
            .expect("a policy that refuses")
    };
    let cases: [(&[&str], &str); 3] = [
        (&["Müller"], "Frau Schmidt-Müller rief zurück."),
        (&[], "Thanks to _Rowan_ for the review."),
        (&[], "会议上Rowan发言了。"),
    ];
    for (known, text) in cases {
        let r = refuse_only(known);
        let (shown, left) = shown(&r, &[text]).remove(0);
        assert_eq!(left, BTreeSet::from(["RareName"]), "{shown}");
    }
}

/// A capitalised word glued to a lower-case letter, directly or through underscores, is part of
/// an identifier: it is neither replaced nor left for the gate.
#[test]
fn a_word_glued_to_a_lower_case_letter_through_underscores_is_not_a_name() {
    let r = redactor(&[], None, true);
    for (text, left) in shown(
        &r,
        &["see snake_case_Identifier and fooBar and x__Quill here"],
    ) {
        for word in ["Identifier", "Bar", "Quill"] {
            assert!(text.contains(word), "{word} was replaced: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

// ---------------------------------------------------------------------------------------------
// The gate disagreeing with the replacement: a batch refused for what was kept on purpose.
// ---------------------------------------------------------------------------------------------

/// A capitalised prefix hyphenated to a known name: the replacement keeps `Co` (it never matches
/// it on its own), and the gate then reads `Co`, never counted, as a rare name and refuses the
/// batch: the shape of issue #71, through a known name.
#[test]
fn the_gate_finds_nothing_left_beside_a_known_name_the_replacement_kept() {
    let r = redactor(&["Jane"], None, true);
    let texts = ["Pairing with Co-Jane today."];
    for (text, left) in shown(&r, &texts) {
        assert_eq!(
            left,
            BTreeSet::new(),
            "the gate refuses what the replacement kept: {text}"
        );
    }
}

/// A document cut at `max_chars` inside the frequent trailer of issue #71: the batch counted
/// `Co-Authored-By` three times in the uncut texts, the model is shown `Co-Auth`, and the gate
/// reads that fragment, never counted, as a rare name and refuses the batch.
#[test]
fn the_gate_finds_nothing_left_of_a_frequent_word_the_cut_splits() {
    let r = redactor(&[], Some(1), true);
    let trailer = "Co-Authored-By: Example Person <person@example.org>";
    let texts = [
        format!("fix: parse the header.\n\n{trailer}"),
        format!("feat: add a flag.\n\n{trailer}"),
        format!("docs: write the guide.\n\n{trailer}"),
    ];
    let mut batch = r.batch();
    for t in &texts {
        batch.reserve(t);
    }
    let cut = &texts[2];
    let keep = cut.find("Co-Authored-By").expect("the trailer") + "Co-Auth".len();
    let shown = batch.replace_prefix(cut, keep);
    assert!(shown.ends_with("Co-Auth"), "{shown}");
    assert_eq!(
        batch.left(&shown),
        BTreeSet::new(),
        "the gate refuses a cut through a word the batch holds three times: {shown}"
    );
}

// ---------------------------------------------------------------------------------------------
// What holds: the skip never swallows the next word, and a rare compound is still replaced.
// ---------------------------------------------------------------------------------------------

/// The skip over a rejected word ends where the word ends: a name after a line break, a double
/// hyphen, a Unicode hyphen, a non-breaking hyphen, an en dash or a dangling hyphen is still
/// replaced, and the frequent word before it is kept as written.
#[test]
fn skipping_a_frequent_word_never_swallows_the_name_after_it() {
    let r = redactor(&[], Some(1), true);
    let word = "Co-Authored-By";
    let texts = [
        format!("a.\n{word}"),
        format!("b.\n{word}"),
        format!("c.\n{word}\nIlse"),
        format!("d.\n{word}--Marit"),
        format!("e.\n{word}\u{2010}Tobiah"),
        format!("f.\n{word}\u{2011}Quill"),
        format!("g.\n{word}\u{2013}Oriel"),
        format!("h.\n{word}-\nSaskia"),
    ];
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    for (text, left) in shown(&r, &refs) {
        assert!(text.contains(word), "the frequent word is kept: {text}");
        for name in ["Ilse", "Marit", "Tobiah", "Quill", "Oriel", "Saskia"] {
            assert!(!text.contains(name), "{name} reaches the model: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

/// A rare name glued to a frequent word makes a new word, seen once: it is replaced whole. So are
/// a frequent first name hyphenated to a rare one, an apostrophe join and a decomposed name.
#[test]
fn a_compound_seen_once_is_replaced_whole_whatever_its_parts() {
    let r = redactor(&[], Some(1), true);
    let texts = [
        "Anna wrote it.",
        "Anna read it.",
        "Anna-Maria signed it.",
        "Co-Jane paired.",
        "Seen by O\u{2019}Neil and Ze\u{301}lie-Anne.",
    ];
    for (text, left) in shown(&r, &texts) {
        for part in ["Maria", "Jane", "Neil", "lie-Anne", "Anne"] {
            assert!(!text.contains(part), "{part} reaches the model: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

/// A higher `rare_limit`: a hyphenated word seen as often as the limit is rare and replaced whole;
/// seen once more, it is kept whole and the gate finds nothing left of it.
#[test]
fn rare_limit_decides_for_the_whole_hyphenated_word() {
    let r = redactor(&[], Some(2), true);
    let two = ["x Jean-Luc.", "y Jean-Luc."];
    for (text, left) in shown(&r, &two) {
        assert!(!text.contains("Jean") && !text.contains("Luc"), "{text}");
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
    let three = ["x Jean-Luc.", "y Jean-Luc.", "z Jean-Luc."];
    for (text, left) in shown(&r, &three) {
        assert!(text.contains("Jean-Luc"), "{text}");
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

// ---------------------------------------------------------------------------------------------
// Word boundaries with hyphens.
// ---------------------------------------------------------------------------------------------

/// A frequent word written with a lower-case head (`co-Authored-By`), and one with a dangling
/// hyphen at the end, are kept as written and the gate finds nothing left of them.
#[test]
fn a_frequent_word_with_a_lower_case_head_or_a_dangling_hyphen_is_kept() {
    let r = redactor(&[], Some(1), true);
    let texts = [
        "a.\nco-Authored-By: x",
        "b.\nco-Authored-By: y",
        "c.\nco-Authored-By: z",
        "d.\nRelease-Notes-",
        "e.\nRelease-Notes-",
        "f.\nRelease-Notes-",
    ];
    for (text, left) in shown(&r, &texts) {
        assert!(
            text.contains("co-Authored-By") || text.contains("Release-Notes-"),
            "the frequent word is kept as written: {text}"
        );
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

/// A rare name at a hyphen boundary that does not join it to another word (a leading hyphen, a
/// trailing hyphen, a hyphen before a digit, a list dash) is still replaced, and nothing is left.
#[test]
fn a_rare_name_at_a_hyphen_that_joins_nothing_is_still_replaced() {
    let r = redactor(&[], Some(1), true);
    let texts = [
        "- Ilse reviewed.",
        "by -Marit today",
        "Tobiah- and others",
        "build Quill-2 shipped",
        "x 12-Oriel y",
    ];
    for (text, left) in shown(&r, &texts) {
        for name in ["Ilse", "Marit", "Tobiah", "Quill", "Oriel"] {
            assert!(!text.contains(name), "{name} reaches the model: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

// ---------------------------------------------------------------------------------------------
// A known name inside a hyphenated word.
// ---------------------------------------------------------------------------------------------

/// A known name at the head of a hyphenated word: the known name is replaced, its rare tail is
/// replaced too, and the gate finds nothing left in what the model is shown.
#[test]
fn a_known_name_at_the_head_of_a_hyphenated_word_leaves_nothing_for_the_gate() {
    let r = redactor(&["Jane"], Some(1), true);
    let texts = [
        "Jane-Rowan joined.",
        "Jane-Street",
        "Jane-Street",
        "Jane-Street",
    ];
    for (text, left) in shown(&r, &texts) {
        assert!(!text.contains("Jane"), "{text}");
        assert!(!text.contains("Rowan"), "{text}");
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

/// A known name that is itself hyphenated, inside a longer hyphenated word: no part of the known
/// name and no rare remainder is shown, and nothing is left.
#[test]
fn a_hyphenated_known_name_inside_a_longer_word_leaves_nothing() {
    let r = redactor(&["Jean-Luc"], Some(1), true);
    for (text, left) in shown(&r, &["Captain Jean-Luc-Picard spoke."]) {
        for part in ["Jean", "Luc", "Picard"] {
            assert!(!text.contains(part), "{part} reaches the model: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

// ---------------------------------------------------------------------------------------------
// refuse_if_left after replace_prefix.
// ---------------------------------------------------------------------------------------------

/// The batch of issue #71, the third document cut by `max_chars`.
fn cut_trailer_batch(r: &Redactor, cut_after: &str) -> (String, BTreeSet<&'static str>) {
    let trailer = "Co-Authored-By: Example Person <person@example.org>";
    let texts = [
        format!("fix: parse the header.\n\n{trailer}"),
        format!("feat: add a flag.\n\n{trailer}"),
        format!("docs: write the guide.\n\n{trailer}"),
    ];
    let mut batch = r.batch();
    for t in &texts {
        batch.reserve(t);
    }
    let cut = &texts[2];
    let keep = cut.find("Co-Authored-By").expect("the trailer") + cut_after.len();
    let shown = batch.replace_prefix(cut, keep);
    let left = batch.left(&shown);
    (shown, left)
}

/// A cut exactly at the end of the frequent word: shown whole, nothing left.
#[test]
fn a_cut_at_the_end_of_a_frequent_word_leaves_nothing() {
    let r = redactor(&[], Some(1), true);
    let (shown, left) = cut_trailer_batch(&r, "Co-Authored-By");
    assert!(shown.ends_with("Co-Authored-By"), "{shown}");
    assert_eq!(left, BTreeSet::new(), "{shown}");
}

/// A cut right after a hyphen of the frequent word: the model is shown `Co-`, and the gate reads
/// `Co`, never counted, as a rare name and refuses the batch.
#[test]
fn a_cut_after_a_hyphen_of_a_frequent_word_leaves_nothing() {
    let r = redactor(&[], Some(1), true);
    let (shown, left) = cut_trailer_batch(&r, "Co-");
    assert!(shown.ends_with("Co-"), "{shown}");
    assert_eq!(
        left,
        BTreeSet::new(),
        "the gate refuses a cut after a hyphen of a frequent word: {shown}"
    );
}

/// A cut through a rare hyphenated word: the part kept is one placeholder, restored to the part
/// kept, and the gate finds nothing left.
#[test]
fn a_cut_through_a_rare_hyphenated_word_is_one_placeholder() {
    let r = redactor(&[], Some(1), true);
    let text = "thanks to Jean-Luc.";
    let mut batch = r.batch();
    batch.reserve(text);
    let keep = text.find("Jean-Luc").expect("the name") + "Jean-L".len();
    let shown = batch.replace_prefix(text, keep);
    assert_eq!(shown, "thanks to [Name-1]");
    assert_eq!(batch.left(&shown), BTreeSet::new(), "{shown}");
    let mut answer = serde_json::json!("[Name-1]");
    assert_eq!(batch.restore(&mut answer), 0);
    assert_eq!(answer, serde_json::json!("Jean-L"));
}

// ---------------------------------------------------------------------------------------------
// Unicode letters.
// ---------------------------------------------------------------------------------------------

/// Cyrillic, Greek and Turkish hyphenated names: seen once, each is one placeholder; seen three
/// times, each is kept whole and the gate finds nothing left.
#[test]
fn a_non_latin_hyphenated_word_is_taken_whole() {
    let r = redactor(&[], Some(1), true);
    for name in ["Анна-Мария", "Ελένη-Μαρία", "İpek-Ayşe"] {
        let once = format!("x {name} y");
        let out = shown(&r, &[once.as_str()]);
        assert_eq!(out[0].0, "x [Name-1] y", "{name}");
        assert_eq!(out[0].1, BTreeSet::new(), "{name}");
        let texts: Vec<String> = (0..3).map(|i| format!("{i} {name}")).collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        for (text, left) in shown(&r, &refs) {
            assert!(text.contains(name), "{text}");
            assert_eq!(left, BTreeSet::new(), "{text}");
        }
    }
}

/// A name joined by a Unicode hyphen (U+2010) or a non-breaking hyphen (U+2011), seen once: no
/// part of it reaches the model and nothing is left.
#[test]
fn a_name_joined_by_a_unicode_hyphen_is_not_shown() {
    let r = redactor(&[], Some(1), true);
    let texts = ["by Jean\u{2010}Luc.", "by Anne\u{2011}Sophie."];
    for (text, left) in shown(&r, &texts) {
        for part in ["Jean", "Luc", "Anne", "Sophie"] {
            assert!(!text.contains(part), "{part} reaches the model: {text}");
        }
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

/// A frequent hyphenated word in decomposed form (NFD), seen three times: kept, nothing left.
#[test]
fn a_frequent_decomposed_hyphenated_word_is_kept() {
    let r = redactor(&[], Some(1), true);
    let word = "Ze\u{301}lie-Anne";
    let texts: Vec<String> = (0..3).map(|i| format!("{i} {word}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    for (text, left) in shown(&r, &refs) {
        assert!(text.contains(word), "{text}");
        assert_eq!(left, BTreeSet::new(), "{text}");
    }
}

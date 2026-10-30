//! `story:credential-key-spellings`: a credential written under a common key spelling is masked
//! before it is stored and before a model sees it. The 25 spellings a downstream instance
//! measured are checked through `mask::mask` alone, through a policy of `classes: [Credential]`,
//! `refuse_if_left: [Credential]` run as a run applies it (mask, scrub, replace, left), and
//! through that policy alone, as a question is shown to the model. Prose and settings that only
//! look like those spellings are kept by all three.
//! Credential-shaped values and names are assembled at run time so no such literal sits in the
//! source.

use cortex_cli::mask::{mask, mask_key};
use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;

fn cat(parts: &[&str]) -> String {
    parts.concat()
}

/// A password-shaped value, built at run time.
fn v() -> String {
    cat(&["Fk9adv1", "Qz7Rm4Lp", "8Wn2Tx"])
}

/// A cloud secret access key's shape, built at run time.
fn aws() -> String {
    cat(&["wJalrXUtnF", "EMI/K7MDEN", "G+bPxRfiCY", "q3Tz9LmW0a"])
}

/// The 25 spellings: a label, the text and the value it holds.
fn variants() -> Vec<(&'static str, String, String)> {
    let s = v();
    let a = aws();
    vec![
        ("DB_PASSWORD=", cat(&["DB_PASS", "WORD=", &s]), s.clone()),
        ("MY_API_KEY=", cat(&["MY_API", "_KEY=", &s]), s.clone()),
        (
            "AWS_SECRET_ACCESS_KEY=",
            cat(&["AWS_SECRET", "_ACCESS_KEY=", &a]),
            a.clone(),
        ),
        (
            "lower db_password=",
            cat(&["db_pass", "word=", &s]),
            s.clone(),
        ),
        ("api_key: (space)", cat(&["api", "_key: ", &s]), s.clone()),
        (
            "aws_secret_access_key = (spaces)",
            cat(&["aws_secret", "_access_key = ", &a]),
            a.clone(),
        ),
        (
            "mixed Db_PassWord=",
            cat(&["Db_Pass", "Word=", &s]),
            s.clone(),
        ),
        (
            "quoted DB_PASSWORD='..'",
            cat(&["export DB_PASS", "WORD='", &s, "'"]),
            s.clone(),
        ),
        (
            "JSON {\"db_password\": \"..\"}",
            cat(&["{\"db_pass", "word\": \"", &s, "\"}"]),
            s.clone(),
        ),
        (
            "JSON-escaped db_password",
            cat(&["{\\\"db_pass", "word\\\":\\\"", &s, "\\\"}"]),
            s.clone(),
        ),
        (
            "JSON-escaped aws secret access key",
            cat(&["{\\\"aws_secret", "_access_key\\\":\\\"", &a, "\\\"}"]),
            a.clone(),
        ),
        (
            "URL query ?api_key=..&",
            cat(&["see https://svc.test/cb?api", "_key=", &s, "&page=2"]),
            s.clone(),
        ),
        (
            "URL-encoded %3Fapi_key%3D..",
            cat(&["next=%2Fcb%3Fapi", "_key%3D", &s]),
            s.clone(),
        ),
        (
            "YAML value on the next line",
            cat(&["aws_secret", "_access_key:\n  ", &a]),
            a.clone(),
        ),
        (
            "Markdown code key `DB_PASSWORD`: ..",
            cat(&["`DB_PASS", "WORD`: ", &s]),
            s.clone(),
        ),
        (
            "Slack bold *AWS_SECRET_ACCESS_KEY*: ..",
            cat(&["*AWS_SECRET", "_ACCESS_KEY*: ", &a]),
            a.clone(),
        ),
        (
            "Markdown bold **api_key**: ..",
            cat(&["**api", "_key**: ", &s]),
            s.clone(),
        ),
        (
            "XML <db_password>..</db_password>",
            cat(&["<db_pass", "word>", &s, "</db_pass", "word>"]),
            s.clone(),
        ),
        (
            "CLI --db-password ..",
            cat(&["psql --db-pass", "word ", &s]),
            s.clone(),
        ),
        ("SECRET_KEY=", cat(&["SECRET", "_KEY=", &s]), s.clone()),
        (
            "DJANGO_SECRET_KEY=",
            cat(&["DJANGO_SECRET", "_KEY=", &s]),
            s.clone(),
        ),
        (
            "AWS_SECRET_KEY=",
            cat(&["AWS_SECRET", "_KEY=", &a]),
            a.clone(),
        ),
        ("PGPASSWORD=", cat(&["PGPASS", "WORD=", &s]), s.clone()),
        ("DB_PASS=", cat(&["DB_PA", "SS=", &s]), s.clone()),
        (
            "url-encoded value aws_secret_access_key=%2B..",
            cat(&["aws_secret", "_access_key=%2B", &a]),
            a.clone(),
        ),
    ]
}

/// The parts of `value` that must not survive: every run of five of its characters.
fn parts(value: &str) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    chars.windows(5).map(|w| w.iter().collect()).collect()
}

fn keeps_a_part(text: &str, value: &str) -> bool {
    parts(value).iter().any(|p| text.contains(p.as_str()))
}

fn policy() -> Redactor {
    let policy = m::RedactionPolicy {
        classes: vec![m::RedactionClass::Credential],
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: Some(vec![m::RedactionClass::Credential]),
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

/// What a run stores and what the model is shown of `text`, and what `left` still finds in the
/// shown text: mask, scrub, replace, left.
fn run_path(r: &Redactor, text: &str) -> (String, String, Vec<&'static str>) {
    let (masked, _) = mask(text);
    let (stored, _) = r.scrub(&masked);
    let mut batch = r.batch();
    batch.reserve(&stored);
    let shown = batch.replace(&stored);
    let left = batch.left(&shown).into_iter().collect();
    (stored, shown, left)
}

/// What the model is shown of a text that was not masked first (a question), and what `left`
/// still finds in it.
fn policy_only(r: &Redactor, text: &str) -> (String, Vec<&'static str>) {
    let mut batch = r.batch();
    batch.reserve(text);
    let shown = batch.replace(text);
    let left = batch.left(&shown).into_iter().collect();
    (shown, left)
}

#[test]
fn each_spelling_is_masked_by_mask_alone() {
    let leaks: Vec<String> = variants()
        .into_iter()
        .filter_map(|(label, text, value)| {
            let (masked, n) = mask(&text);
            let (key, _) = mask_key(&text);
            (keeps_a_part(&masked, &value) || keeps_a_part(&key, &value) || n == 0)
                .then(|| format!("{label}: {masked} ({n})"))
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "{} of 25 leak:\n{}",
        leaks.len(),
        leaks.join("\n")
    );
}

#[test]
fn each_spelling_is_masked_on_a_runs_policy_path() {
    let r = policy();
    let leaks: Vec<String> = variants()
        .into_iter()
        .filter_map(|(label, text, value)| {
            let (stored, shown, left) = run_path(&r, &text);
            (keeps_a_part(&stored, &value) || keeps_a_part(&shown, &value))
                .then(|| format!("{label}: stored {stored} shown {shown} left {left:?}"))
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "{} of 25 leak:\n{}",
        leaks.len(),
        leaks.join("\n")
    );
}

/// A question is not masked first: the policy alone hides the value, or `left` names
/// `Credential` and the question is refused.
#[test]
fn each_spelling_is_hidden_or_refused_by_the_policy_alone() {
    let r = policy();
    let leaks: Vec<String> = variants()
        .into_iter()
        .filter_map(|(label, text, value)| {
            let (shown, left) = policy_only(&r, &text);
            (keeps_a_part(&shown, &value) && !left.contains(&"Credential"))
                .then(|| format!("{label}: shown {shown} left {left:?}"))
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "{} of 25 leak:\n{}",
        leaks.len(),
        leaks.join("\n")
    );
}

/// Text that only looks like a credential's spelling, kept by `mask`, `mask_key`, a run's policy
/// path and the policy alone, with nothing left.
#[test]
fn look_alike_settings_and_prose_are_kept() {
    let id = cat(&["Prod", "KeyId", "0042"]);
    let kept = [
        // The key id of an access key pair names the key; the secret is the other half.
        cat(&["access_key", "_id = ", &id]),
        cat(&["AWS_ACCESS_KEY", "_ID=", &id]),
        // A word that starts with `secret` or `password` and goes on is no credential's name.
        cat(&["secret", "_santa: ", "Alice2026"]),
        cat(&["password", "_policy: strict"]),
        "The password_policy: strict mode needs twelve characters.".to_string(),
        "The secret key is rotated monthly by the platform team.".to_string(),
        // A flag naming a file, a flag without a value, and a flag followed by a word of prose.
        cat(&["psql --password", "-file /etc/app/db.pass"]),
        cat(&["pass the --pass", "word option to prompt"]),
        cat(&["login --pass", "word --host db.internal"]),
        // An element that names a hint, and a placeholder with no closing tag.
        cat(&["<password", "_hint>First pet</password", "_hint>"]),
        cat(&["usage: login <user> <pass", "word> required"]),
        // A Markdown heading or label with no value.
        cat(&["**Pass", "word**\n\nChoose a long one."]),
        cat(&["## Pass", "word\n\nRotated every month."]),
        // A file that holds passwords is no password.
        cat(&["PGPASS", "FILE=~/.pgpass"]),
        // `pass` alone is a word.
        "Mountain pass: Stelvio, then bypass: maintenance".to_string(),
    ];
    let r = policy();
    for text in kept {
        assert_eq!(mask(&text), (text.clone(), 0), "mask: {text}");
        assert_eq!(mask_key(&text), (text.clone(), 0), "mask_key: {text}");
        let (stored, shown, left) = run_path(&r, &text);
        assert_eq!(
            (stored.as_str(), shown.as_str()),
            (text.as_str(), text.as_str()),
            "run"
        );
        assert!(left.is_empty(), "run left {left:?}: {text}");
        let (shown, left) = policy_only(&r, &text);
        assert_eq!(shown, text, "policy alone");
        assert!(left.is_empty(), "policy alone left {left:?}: {text}");
    }
}

/// A bold label with its colon inside the markup (`**Password:** <value>`), and a name with a
/// leading `_` (`self._password = <value>`), is masked with the
/// label kept, by `mask` and by the policy alone.
#[test]
fn a_bold_label_with_its_colon_inside_is_masked() {
    let s = v();
    let r = policy();
    for label in [
        cat(&["**Pass", "word:** "]),
        cat(&["__db_pass", "word:__ "]),
        cat(&["self._pass", "word = "]),
    ] {
        let text = format!("{label}{s}");
        assert_eq!(
            mask(&text),
            (format!("{label}[masked:secret-assignment]"), 1),
            "{text}"
        );
        let (shown, _) = policy_only(&r, &text);
        assert_eq!(
            shown,
            format!("{label}[masked:secret-assignment]"),
            "{text}"
        );
    }
}

/// Each spelling, masked once, is masked to itself and counted no more.
#[test]
fn masking_a_spelling_twice_changes_nothing_more() {
    for (label, text, _) in variants() {
        let (once, _) = mask(&text);
        assert_eq!(mask(&once), (once.clone(), 0), "{label}");
        let (once, _) = mask_key(&text);
        assert_eq!(mask_key(&once), (once.clone(), 0), "{label}");
    }
}

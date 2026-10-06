---
format: aep.planning-md/3
id: review-result:adversary-record-field-lookups-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: record-field-lookups'
relations:
- reviews: story:record-field-lookups
revision: 1
---
```
unit: story:record-field-lookups (U1, wave 20261006c), uncommitted working tree at ~/.local/state/worktree/trees/b10x/cortex/cortex-w10-a on base a80c9f6
verdict: NEEDS-CHANGE
cases: executed 336→344, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (part 6)
needs-coordinator: none
```

**1. Diff of my additions**

`git diff --stat` does not list untracked files. `git status --short -- tests/` shows:
```
 M tests/file_records.rs              (the implementor's change, not mine)
?? tests/record_lookups_adversary.rs  (mine: 8 cases, test code only)
```
I changed no non-test path.

**2. Cases added** (`tests/record_lookups_adversary.rs`). I ran each red case on its own first.

| Case | Now | Red output, from the run of that case alone |
|---|---|---|
| `a_labelled_mention_is_mapped_by_its_id` | red | `panicked at tests/record_lookups_adversary.rs:142:5:` … `Ana: hi <@U2\|ben>` |
| `a_lookup_file_with_a_byte_order_mark_is_read` | red | `left: (1, Some("fetch-failed")) right: (0, Some("ran"))` with reason `lookup …/people.json: not a JSON object of strings: expected value at line 1 column 1` |
| `a_blank_name_in_the_lookup_keeps_the_id` | red | `panicked at tests/record_lookups_adversary.rs:185:5:` … document text `: hi @  ` |
| `a_lookup_inside_the_sources_glob_is_not_a_document` | green | the lookup stays out of the documents |
| `an_absolute_and_a_symlinked_lookup_are_read` | green | both lookups are read |
| `a_name_with_mention_or_template_syntax_is_not_expanded_again` | green | the name is inserted as written |
| `a_mapped_name_is_redacted_like_any_other_name` | green | RareName with `rare_limit: 1` replaces both Ana and Ben |
| `only_the_first_non_empty_fallback_is_the_text` | green | pins "the first non-empty one" (see finding 4) |

**3. Suite run, after the cases existed**

Command: `cargo test --locked --no-fail-fast` in the root package, with the brief's env. Exit was `EXIT=101`. All 22 other test binaries passed, including `file_records` 17/17 and the lib tests 136/136. The new file showed:
```
test result: FAILED. 5 passed; 3 failed; ... (record_lookups_adversary)
```
The run executed 344 cases. Without my 8 that is 336. That this equals the implementor's `task check` count of 344 is a coincidence: `task check` covers a different scope. Log: `~/.cache/cortex-wave-20261006c/a/adversary1/suite.log`.

**4. Findings**

| # | file:line | Verdict / origin | Severity | Finding | What reaches it |
|---|---|---|---|---|---|
| 1 | `src/sources.rs:791` | NEEDS-CHANGE / introduced | warning | `<@U2\|ben>` looks up the key `U2\|ben`, so the mention stays a raw id even though the lookup holds `U2`. The story says "every `<@id>`". Fix: cut the key at the first `\|` before the lookup. | Older chat exports write mentions with a label. |
| 2 | `src/sources.rs:694` | NEEDS-CHANGE / introduced | warning | A lookup file that starts with a UTF-8 BOM fails the source's whole run (`fetch-failed`). Record files strip the BOM (`file_records`, `strip_prefix(b"\xef\xbb\xbf")`). Fix: strip it the same way in `read_lookup`. | Editors and shell tools on Windows save JSON with a BOM. |
| 3 | `src/sources.rs:913` (and `:791`) | CONFIRMED / introduced | note | When a name in the lookup is blank, the author prefix becomes `: ` and the mention becomes `@  `, so the id is lost. A blank author value is treated as no author (`scalar`). Fix: treat a blank name as absent and keep the id. | A lookup built from display names, which are often empty. |
| 4 | `tests/file_records.rs` (`fallback_text_stands_in_for_an_empty_text_and_both_empty_yield_nothing`) | CONFIRMED / introduced | note | The unit's case has only one non-empty fallback, so it would pass if the code took the last fallback or joined them all. My case `only_the_first_non_empty_fallback_is_the_text` would catch that change. I argued this from the code and did not run it against a mutated copy: disk was at 5.8 GB. | The test suite only. |

These cover the uncommitted working tree on top of a80c9f6.

**5. Attacked and not broken**
- **Lookup inside the source's own glob (`*`, JsonLines):** it is not ingested, because it has no `id`. A pretty-printed lookup would only add "line N not JSON" entries to `skipped`. With `WholeFile` it would be ingested, but `lookup` does nothing there; I did not test that.
- **`../`, absolute and symlinked lookup paths:** all are read, as the design says ("resolved the way `paths` is"). The spec author controls them.
- **`<@U2>` at the edges of the text, adjacent mentions, `<@>` and an unclosed `<@`:** all correct (the unit's own test and reading the code).
- **Nested `<@<@U2>>`:** gives `<@@Ben>`, which is harmless.
- **Very large input:** the lookup is a BTreeMap, read once. `mentions` is quadratic only for a huge `<@` run with no `>`, and std's search is fast, so this was not worth a case.
- **Names containing `<@U2>` or `{text}`:** they are not expanded again (green case).
- **Redaction:** a mapped name is still redacted (green case). Residue: the author prefix repeats a person's name in every message they wrote, so RareName stops treating a frequent poster's name as rare. The design accepts this ("same policy as names already in the text").
- **`text` that renders only whitespace:** the fallback is used (green case).
- **Thread context and unknown authors:** context holds the text after mapping, and an unknown author stays as `U9:`.
- **Existing specs without the new fields:** `mentions` with an empty lookup returns its input unchanged, the new fields are serialized only when present, and `ContentHash` is the only change-detection mode. The 17 `file_records` tests and 22 `structured` tests are green.

**6. Paths written outside the worktree** (all in the assigned scratch directory)
- `~/.cache/cortex-wave-20261006c/a/adversary1/red-labelled.log`
- `~/.cache/cortex-wave-20261006c/a/adversary1/red-a_lookup_file_with_a_byte_order_mark_is_read.log`
- `~/.cache/cortex-wave-20261006c/a/adversary1/red-a_blank_name_in_the_lookup_keeps_the_id.log`
- `~/.cache/cortex-wave-20261006c/a/adversary1/suite.log`
- the test binary `record_lookups_adversary-*` in `~/.cache/b10x-target/cortex-w10-a/debug/deps/`

The session lease `adversary-w10c-u1-p1` was acquired and has been released.

**7. Findings block**
```findings
[
 {
  "file": "src/sources.rs",
  "line": 791,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "a labelled mention <@U2|ben> is looked up by the whole \"U2|ben\" and stays unmapped although the lookup holds U2"
 },
 {
  "file": "src/sources.rs",
  "line": 694,
  "category": "boundary",
  "severity": "warning",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "a lookup file with a UTF-8 byte-order mark fails the source's fetch, while record files of the same source have the mark stripped"
 },
 {
  "file": "src/sources.rs",
  "line": 913,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a blank name in the lookup replaces the author with an empty prefix \": \" and a mention with \"@  \", erasing the id"
 },
 {
  "file": "tests/file_records.rs",
  "category": "mutant",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the fallback acceptance case has one non-empty fallback, so taking the last or joining all fallbacks would stay green"
 }
]
```
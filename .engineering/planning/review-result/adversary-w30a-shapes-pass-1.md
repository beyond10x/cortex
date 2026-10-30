---
format: aep.planning-md/3
id: review-result:adversary-w30a-shapes-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit shapes (wave 20261030a)'
relations:
- reviews: story:structured-children-follow-rule
- reviews: story:record-key-template
- reviews: story:child-text-and-filter
revision: 1
---
unit: `shapes` of cortex wave 20261030a. Findings cover `unit/w30a-shapes` at 48347819, plus my test commit 74af6567.
verdict: NEEDS-CHANGE
cases: executed 369→372, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: none

1. Diff stat, 48347819..74af6567:
```
 tests/w30a_shapes_adversary.rs | 150 +++++++++++++++++++++++++++++++++++++++++
 1 file changed, 150 insertions(+)
```
Only the test file changed. The commit was made by `b10x-bot[bot]`, who is both author and committer. The gates scan of the file and of the commit message was clean.

2. Cases in `tests/w30a_shapes_adversary.rs`. All three are red now. I ran this file alone first, with `cargo test --test w30a_shapes_adversary`; its first run gave `0 passed; 3 failed`.
   - **`:79` `adding_an_empty_key_to_a_source_with_seen_documents_is_refused`.** It checks that `cortex update` refuses `key: ""` on a source that has seen documents. Red output:
     ```
     update accepted `key: ""` on a source with seen documents: {"command":"update","detail":{"added_sources":[],"name":"empty"},"outcome":"updated"}
     the next run: {... "documents_applied":0, ... "skipped":["tracker.messages.list: 2 records skipped: no value for key ``"] ...}
     ```
   - **`:113` `a_key_that_is_no_template_is_refused_when_the_spec_is_read`.** It checks that `create` refuses a `key` of `"{input.channel}:{ts"`, `"{ts}}"` or `"channel"`. Red output (first key):
     ```
     key "{input.channel}:{ts" was accepted: {"command":"create",...,"outcome":"created"}
     its first run: {... "documents_applied":0, ... "skipped":["tracker.messages.list: 2 records skipped: no value for key `{input.channel}:{ts`"] ...}
     ```
   - **`:137` `an_equals_rule_on_a_large_integer_does_not_follow_a_parent_with_a_neighbouring_one`.** It checks that `follows(equals owner_id 9007199254740993)` is false for a parent whose `owner_id` is 9007199254740992. Red output:
     ```
     equals 9007199254740993 followed {"owner_id":9007199254740992}
     ```

3. Suite run, after the cases existed:
   - Command: `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test --no-fail-fast --lib --test structured_children --test structured_children_adv --test connectors_walks --test spec_forms --test rekey_adversary --test rekey_adversary_pass2 --test w30a_shapes_adversary`
   - Results:
     ```
     lib 292 passed | connectors_walks 36 passed | rekey_adversary 4 passed | rekey_adversary_pass2 4 passed (2 ignored)
     spec_forms 11 passed | structured_children 11 passed | structured_children_adv 11 passed
     w30a_shapes_adversary: 0 passed; 3 failed
     EXIT=101
     ```
   - The before count of 369 comes from the same run with `w30a_shapes_adversary` left out.

4. Findings:

   | file:line | verdict | origin | what was measured | what reaches it |
   |---|---|---|---|---|
   | `src/main.rs:1611` | NEEDS-CHANGE | introduced | `keyed_by` reads an absent `key` and `key: ""` as the same field, so `update` lets the change through. From the next run on, every record is skipped and no seen document is refreshed again. | Any `cortex update` that adds `key: ""`. `model_map.rs:577` maps the empty string to `Some("")`. |
   | `src/model_map.rs:577` / `src/spec.rs:421` | NEEDS-CHANGE | introduced | Nothing checks a `key` template when the spec is read. Alias templates use the same grammar (`render_template`) and are refused at load. A typo in `key` therefore creates a source that skips every record on every run, and the skip message blames the records. | A typo in `key` in any connectors spec. Fix: run `structured::template_refusal` on `ConnectorsSource.key` in `spec::check`, beside `alias_template_refusal`. An empty `key` would then be refused too, which also closes the first row. |
   | `src/sources.rs:1573` | INFEASIBLE | introduced | `equals` compares numbers as `f64`, so two integers above 2^53 that differ count as equal. This contradicts the spec's "numbers compare by value". | No configuration found that uses `equals` on an integer above 2^53. It holds, but I did not show anyone reaching it. |

   Judgement only, not run as a case:
   - **`src/main.rs:1608`.** `keyed_by` does not include `inputs`. With a `{input.…}` template, the input values now become part of document keys, so changing an input value re-keys documents without `rekey-refused`. Respelling `{channel: C1}` as `{channel: general}` would do it. Severity: note. Origin: introduced.

5. Attacked and could not break:
   - **`within_days`:**
     - Exactly N days is followed.
     - A future time is followed.
     - A text with a `+01:00` offset is parsed.
     - A time with no zone, a non-time string and a JSON number do not match, as the spec says.
     - A Jira-style `+0000` offset (no colon) does not match. That limit is in the existing date parser, not this unit.
   - **`at_least`:** a string number, a float, a negative and the equality boundary all behave as specified.
   - **`equals`:** `null` against a missing field, `"1"` against `1`, and `1` against `1.0` all behave as specified.
   - **Skipped parents under `dropped: Supersede`:** the parent is held like a failed child call. The listing cache (`ended_listing`) hashes the held prefixes too. When a `changes` child is followed but the `tags` child is not, the links are held, not ended.
   - **Connector child-failure count:** a parent its rule skips leaves the count unchanged, and nothing is cleared wrongly.
   - **Masking:** rendered keys go through the same `mask_key` and `scrub_key` path as plain keys, and child text goes through `mask` and `scrub` with the rest of the document text.
   - **`rekey-refused`:** adding a non-empty `key`, removing it, and using an equivalent spelling (`{$.ts}`) are all refused.
   - **Old `…:null` keys:** existing instances stop refreshing those documents, as the CHANGELOG says.
   - **Same key twice in one run:** the second record is dropped without a report line, as before; the docs say "counts once".
   - **Child `text` rendering:** rendered fields are no longer JSON-escaped, so a child field can contain line breaks or the prompt's document delimiter. A source's own `text` templates already allow this. Not run.

6. Paths written outside the worktree (`~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/shapes/`):
   - `adv-alone.log`
   - `suite.log`
   - `commit-msg.gmWX3Z`
   - the directory itself, which I created

   The builds used the worktree's own `target/`. The test harness also writes its temporary worlds under `CARGO_TARGET_TMPDIR`, or failing that `~/.cache/claude-tmp/cortex-e2e*`.

```findings
- file: src/main.rs
  line: 1611
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: keyed_by treats an absent key and key "" as equal, so cortex update accepts key "" on a seen source and every later run skips every record
- file: src/model_map.rs
  line: 577
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a ConnectorsSource key that is no template (unclosed brace, stray brace, no placeholder) is accepted at load, unlike alias templates of the same grammar, and every run then skips every record
- file: src/sources.rs
  line: 1573
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: equals compares numbers as f64, so integers above 2^53 that differ match, contradicting "numbers compare by value"; no configuration reaching it was found
- file: src/main.rs
  line: 1608
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: keyed_by omits inputs although a key template's input placeholders make input values part of document keys, so changing an input re-keys documents without rekey-refused
```

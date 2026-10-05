---
format: aep.planning-md/3
id: review-result:adversary-structured-source-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: structured-source (wave 20261005e)'
relations:
- reviews: story:structured-source
revision: 1
---
unit: U1 story:structured-source, uncommitted working tree on b920177 in ~/.local/state/worktree/trees/b10x/cortex/cortex-w4-structured
verdict: NEEDS-CHANGE
cases: executed 184→195, red 9 (the gate stops at the first red test binary, so cortex-docs' 8 cases did not run after my cases; 184 is from the implementor's gate.log)
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 24 paths under ~/.cache/cortex-wave-20261005e/structured/adversary-1/ (listed in part 6), plus the shared build dir ~/.cache/b10x-target/cortex-w4-structured
needs-coordinator: findings 1, 3 and 4 need a design decision. EKR 0.0.30 `apply-extraction` matches a node through any alias two entities share. Namespacing the id alias fixes 3 and 4, but two people with the same name still merge (1) while the name stays a matching alias.

**1. diff --stat.** `git --no-pager diff --stat` lists only the implementor's tracked files (src/lib.rs, src/run.rs, src/sources.rs, tests/e2e.rs, website/docs/operating.md, website/docs/spec-file.md). My only change is a new, untracked test file, `tests/structured_adversary.rs`, which `git status` shows as `??`. I changed no non-test path.

**2. Cases** (`tests/structured_adversary.rs`, headed "ADVERSARY CASES"). Each one was run alone first; logs are in `adversary-1/cases/<name>.log`.

| case (line) | asserts | now | red output, run alone |
|---|---|---|---|
| `adv_two_records_with_one_name_and_two_ids_are_two_nodes` :212 | two "John Smith" records with ids P-1 and P-2 become two nodes | red | `assertion left != right failed: two people merged into one node` (left = right = `01a10c79-d752-…`) |
| `adv_an_id_equal_to_another_records_alias_does_not_merge_them` :265 | a record with id `ada` and another with handle `ada` become two nodes | red | `two people merged through an id/alias collision` |
| `adv_two_sources_with_overlapping_numeric_ids_keep_their_people_apart` :289 | id `1` from source A and id `1` from source B become two nodes | red | `two people merged through a shared numeric id` |
| `adv_an_id_matching_an_irreversible_rule_never_reaches_the_store` :408 | with an email rule that has a `replacement` and records keyed by email, the email never reaches the store | red | `an irreversible match reached the store`; stored `"locator":"record:directory:people.list:ada.lovelace@example.org"` and evidence text `"Source: record:directory:people.list:ada.lovelace@example.org…"` |
| `adv_two_ids_scrubbed_to_one_replacement_stay_two_nodes` :442 | two email-keyed records under that rule become two nodes | red | `two people merged through a shared redaction replacement`; Grace's role is stored on "Ada Lovelace" |
| `adv_a_changed_property_leaves_one_value` :468 | role changes from Engineer to Lead and leaves one active value | red | `left: ["Lead", "Engineer"] right: ["Lead"]` |
| `adv_a_root_paging_param_reads_every_page` :532 | `paging.param: "$.page"` reads pages 1 to 3 | red | `the page with Maintainer was never read and nothing says so (…"stopped":null…)`; second call sent `--input-json {"$":{"page":2}}`, with no third call |
| `adv_value_of_200000_characters` :644 | a 200 000-character property value applies | red | `"Ada" lost role "yyyy…" (…"documents_applied":1,…"parts_rejected":1…): []` |
| `adv_hostile_and_unusual_values_round_trip_as_text` :657 | the value cases together | red | `parts_rejected left: 1 right: 0`. The split cases at :598 to :644 trace this to the 200k value only |
| :235, :322, :341, :370, :495, :598, :612, :623, :634, :704 | rename, self-relation, relation by id across runs and batches, second mapping of the same type, YAML/tag/unicode/NUL/number values, duplicate inputs | green | — |

I wrote the paging case first with two pages and it passed. My stand-in was too lenient: it read `"page":2` inside `{"$":{...}}`. I rewrote it with three pages, and that version is the red one above.

**3. Suite run**, after the cases existed. A clippy fix to my own file (`[ada.clone()]` → `[ada]`) came first.
```
export CARGO_TARGET_DIR=~/.cache/b10x-target/cortex-w4-structured RUSTC_WRAPPER=sccache CORTEX_TEST_EKR=… TMPDIR=…/adversary-1/tmp
task check > …/adversary-1/gate.log 2>&1; echo "EXIT=$?" >> …/gate.log
test result: ok. 3 passed; … (tests/structured.rs)
test result: FAILED. 10 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.76s
error: test failed, to rerun pass `-p cortex-cli --test structured_adversary`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```
Every other binary is unchanged: 75, 1, 1, 21, 8, 9, 13, 9, 11, 4, 6, 15 and 3 passed.

**4. Findings** (they cover the working tree on b920177)

| # | file:line | verdict | origin | what was measured / what reaches it / named fix |
|---|---|---|---|---|
| 1 | src/structured.rs:47 | NEEDS-CHANGE | introduced | **Measured:** two records with the same name and different ids are merged into one node (case :212). **Reaches it:** any directory where two people share a name, through the documented mapping. **Fix:** needs the design decision above. |
| 2 | src/sources.rs:360 | NEEDS-CHANGE | introduced | **Measured:** the key is built from the raw id, which is never cleaned, so an email id reaches the store in the evidence locator and the `Source:` header (case :408). **Reaches it:** records keyed by email plus a redaction rule with a `replacement`. spec-file.md:237 promises this cannot happen. **Fix:** derive the key and locator from a digest of the raw id. |
| 3 | src/structured.rs:52 | NEEDS-CHANGE | introduced | **Measured:** the id alias is taken after cleaning, so every id scrubbed to `[email]` shares one alias and all those records merge into one node (case :442). **Reaches it:** the same setup as 2. **Fix:** an identity alias built from a digest of the raw id and namespaced by source. |
| 4 | src/structured.rs:52 | CONFIRMED | introduced | **Measured:** the bare id is a global alias, so overlapping numeric ids across sources (:289) or an id equal to another record's handle (:265) merge two people. **Reaches it:** two sources of one node type, which the docs allow; I found no instance of that setup in use. **Fix:** namespace the id alias (`<adapter>:<operation>:<id>`). |
| 5 | src/sources.rs:521 | NEEDS-CHANGE | introduced | **Measured:** `at` now strips `$.` but `set_at` (:156) does not. A `$.page` paging param sends `{"$":{"page":2}}`, stops after two calls and notes nothing, so the window advances past unread pages (case :532). **Reaches it:** spec-file.md:217 says paths may start at `$`, and `at` is shared with `kind: connectors`. **Fix:** strip `$.` in `set_at` too, or reject `$` in `param`. |
| 6 | website/docs/spec-file.md:226 | CONFIRMED | introduced | **Measured:** the doc says a property "is set to the value", but a changed value adds a second active assertion and the old one stays (case :468). The implementor's test at tests/structured.rs:271 only checks that "Lead" is present, so it cannot catch this. **Reaches it:** any record whose value changes. **Fix:** document that the old value stays, or retract it; and assert the old value is gone at :271. |
| 7 | website/docs/spec-file.md:240 | CONFIRMED | introduced | **Measured:** a record is never cut, and a 200k-character value is rejected by `ekr` (`parts_rejected: 1`). The record is still counted as applied and marked seen, so it is never retried (case :644). **Reaches it:** a long text field mapped as a property. **Fix:** document the limit, or do not mark a record seen when one of its parts was rejected. |

**5. What I attacked and could not break**
- A renamed record stays one node.
- A self-relation is applied and does not cost the batch.
- A relation by id to a record from an earlier run, or from a later batch of the same run, reaches that record's node.
- Two mappings of one node type with different properties both apply.
- YAML syntax, `!Relation` text, unicode, bidi and NUL characters, number names and boolean values round-trip as text.
- Records answered twice apply once.
- A record cannot inject EKR operations: record content only ever lands in string values, never in keys or tags (`extract::tagged`).

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261005e/structured/adversary-1/`: `gate.log`, `list.log`, `run-each.sh`, `tmp/` (empty), and 19 files `cases/<case>.log`, one per case in part 2
- `~/.cache/b10x-target/cortex-w4-structured` (shared build dir, reused)

```findings
[
 {
  "file": "src/structured.rs",
  "line": 47,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "two records with one name and two ids are merged into one node because the name is a resolving alias"
 },
 {
  "file": "src/sources.rs",
  "line": 360,
  "category": "contract-drift",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "the raw record id builds the evidence key, so an id matching an irreversible rule reaches the store in the locator and Source header"
 },
 {
  "file": "src/structured.rs",
  "line": 52,
  "category": "boundary",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "ids scrubbed to one replacement text share that alias and every such record merges into one node"
 },
 {
  "file": "src/structured.rs",
  "line": 52,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the bare id is a global alias, so overlapping ids across sources or an id equal to another record's handle merge two people"
 },
 {
  "file": "src/sources.rs",
  "line": 521,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "at() strips a leading $ but set_at() does not, so a $.page paging param stops after two calls, reads nothing new and notes nothing unread"
 },
 {
  "file": "website/docs/spec-file.md",
  "line": 226,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a changed property value leaves the old value active beside the new one, and tests/structured.rs:271 asserts only presence"
 },
 {
  "file": "website/docs/spec-file.md",
  "line": 240,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "an uncut 200000-character value is rejected by ekr while the record is counted applied and marked seen, so it is never retried"
 }
]
```
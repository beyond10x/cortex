---
format: aep.planning-md/3
id: review-result:adversary-w30a-shapes-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit shapes (wave 20261030a)'
relations:
- reviews: story:structured-children-follow-rule
- reviews: story:record-key-template
- reviews: story:child-text-and-filter
revision: 1
---
unit: `shapes` of cortex wave 20261030a, on branch `unit/w30a-shapes`; findings cover 4523f19c (the correction 19d7a75a and f4032ed0, plus my test commit)
verdict: NEEDS-CHANGE
cases: executed 374→378, red 4
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: none

1. Diff stat, f4032ed0..4523f19c:
```
 tests/w30a_shapes_adversary_pass2.rs | 227 +++++++++++++++++++++++++++++++++++
 1 file changed, 227 insertions(+)
```
   Only the test file changed. `b10x-bot[bot]` is author and committer, and the gates scan of the file and of the message was clean.

2. Cases in `tests/w30a_shapes_adversary_pass2.rs`, all red now. The first run was this file alone (`cargo test --test w30a_shapes_adversary_pass2`): `0 passed; 4 failed`.
   - **`:117` `adding_a_channel_under_an_input_channel_key_is_not_a_rekey`.** The source is keyed `{input.channel}:{ts}` and has run once on `[{channel: C1}]`. The case adds `{channel: C2}`, expects `updated`, and expects the next run to apply only C2's 2 documents.
     ```
     adding input C2 re-keys no document of C1: {"command":"update","detail":{"field":"inputs","name":"addchan","source":"chat"},"outcome":"rekey-refused"}
       left: (1, Some("rekey-refused"))
      right: (0, Some("updated"))
     ```
   - **`:149` `changing_an_input_field_the_key_does_not_name_is_not_a_rekey`.** Same key; `limit: 10` becomes `limit: 20`.
     ```
     `limit` is in no key: {"command":"update","detail":{"field":"inputs","name":"limit","source":"chat"},"outcome":"rekey-refused"}
     ```
   - **`:176` `a_key_naming_an_input_field_no_input_holds_is_refused_when_the_spec_is_read`.** Key `{input.chanel}:{ts}` over inputs that hold only `channel`.
     ```
     key `{input.chanel}` over inputs holding only `channel` was accepted: {...,"outcome":"created"}
     its first run: {..."documents_applied":0,..."skipped":["tracker.messages.list: 2 records skipped: no value for key `{input.chanel}:{ts}`"],...}
     ```
   - **`:202` `a_whole_number_written_with_a_fraction_compares_exactly_with_a_large_integer`.** `equals` and `at_least` with 9007199254740993, on a parent `{"n": 9007199254740992.0}`, which is exact in f64.
     ```
     assertion `left == right` failed: (equals 9007199254740993, at_least 9007199254740993) on {"n":9007199254740992.0}
       left: (true, true)
      right: (false, false)
     ```

3. Suite run, after the cases existed:
   - Command: `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test --no-fail-fast --lib --test structured_children --test structured_children_adv --test connectors_walks --test spec_forms --test rekey_adversary --test rekey_adversary_pass2 --test w30a_shapes_adversary --test w30a_shapes_adversary_pass2`
   - Results:
     ```
     lib 293 ok | connectors_walks 37 ok | rekey_adversary 4 ok | rekey_adversary_pass2 4 ok (2 ignored)
     spec_forms 11 ok | structured_children 11 ok | structured_children_adv 11 ok | w30a_shapes_adversary 3 ok
     w30a_shapes_adversary_pass2: 0 passed; 4 failed
     EXIT=101
     ```
   - The before count of 374 is the same run without `w30a_shapes_adversary_pass2`.

4. Findings:

   | file:line | verdict | origin | what was measured | what reaches it |
   |---|---|---|---|---|
   | `src/main.rs:1620` | NEEDS-CHANGE | introduced | D26 puts the whole `inputs` list into `keyed_by`. So adding a channel, or changing an input field the key does not name, is refused as `rekey-refused`, though no seen document's key changes. There is no override, so an instance keyed by channel can never take another channel without being created again. | The story's own use: `{input.channel}:{ts}` exists for several channels (acceptance 1), and adding one later is the ordinary next `cortex update`. Fix: refuse only when an existing input's values at the paths the key names change, or an input is respelled. An appended input, or a change to a field the key does not name, is not a re-key. |
   | `src/spec.rs:457` | CONFIRMED | introduced | D24 checks the key's grammar only. A placeholder `{input.x}` whose `x` no input holds passes. Inputs are fixed in the spec (`with_window` only replaces `{since}`-style values and adds no fields), so this is knowable when the spec is read. Every run then skips every record. | A typo in an input placeholder, the same class as pass-1 finding 2. The skip line does name the template. Fix: in `key_template_refusal`, refuse an `{input.…}` path that some input does not hold as a scalar. |
   | `src/sources.rs:1609` | INFEASIBLE | introduced | `Num::compare` casts an `Int` to `f64` when the other side is a `Float`. So 9007199254740993 equals, and is at least, 9007199254740992.0. This contradicts the docs: "numbers compare by value, whole numbers exactly". | No configuration found where a source sends integers above 2^53 written with a fraction or exponent. Fix: compare an integer with a finite float exactly, for example by converting a whole float to i128 or comparing against its floor and ceiling. |

5. Attacked and could not break:
   - **Load check against rendering:** `key_template_refusal` and `render_template` share `template_parts`, so the check refuses only templates that could never render. `{$.ts}`, nested paths and `{input.x}` alone are all accepted, and every existing test passes.
   - **Running loads (D19):** `check` also runs on frozen copies, but `key` does not exist at base 36e6b989 (v0.6.1-79), so no released instance stores a key that could stop loading.
   - **`adopt`:** it goes through `load_given`, then `parse`, then `check`, so it is checked too.
   - **`Num` edge cases:**
     - i128 cannot overflow from i64 or u64.
     - A u64 above i64::MAX is read through `as_u64`.
     - `-0` is read as f64 -0.0 and equals 0.
     - `1e2` equals `100`.
     - A decimal string falls to kind equality.
     - `at_least` agrees with `equals` at the boundary.
     - The spec path keeps every digit (`model_map::json` writes `n.to_string()`).
   - **`template_names_input`:**
     - `{ input.x }` with spaces is refused by the grammar.
     - `{$.input.x}` and `{input}` are detected.
     - The grammar has no escape for a brace.
     - A record field named `input` is shadowed by the input in `record_id`, which is consistent with the detection.
   - **Adding `{input.…}` to a key on a seen source:** refused as a `key` change.
   - **Pass-1 assertions:** none was relaxed. f4032ed0 only replaced the call through `World::cortex` with `status()` and added the `.key:` assertion.

6. Paths written outside the worktree, all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/shapes/`:
   - `pass2-alone.log`
   - `pass2-suite.log`
   - `commit-msg-pass2.opxJ2G`

   The builds used the worktree's own `target/`. The test harness also writes its worlds under `~/.cache/claude-tmp/cortex-e2e*`. I took a worktree session lease named `adversary-w30a-shapes-pass2` and released it.

```findings
- file: src/main.rs
  line: 1620
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: keyed_by compares the whole inputs list under an {input.…} key, so adding a channel or changing an input field the key does not name is refused as rekey-refused with no override, though no seen key changes
- file: src/spec.rs
  line: 457
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: key_template_refusal checks grammar only, so a key placeholder {input.x} naming a field no input holds is accepted at load and every run skips every record
- file: src/sources.rs
  line: 1609
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Num::compare casts an integer to f64 against a float, so equals and at_least treat 9007199254740993 as equal to 9007199254740992.0, contradicting "whole numbers exactly"
```

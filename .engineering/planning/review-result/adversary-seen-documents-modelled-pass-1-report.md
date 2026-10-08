---
format: aep.planning-md/3
id: review-result:adversary-seen-documents-modelled-pass-1-report
kind: review-result
status: archived
title: 'Adversary pass 1: seen-documents-modelled (wave 20261005c, by reading; disk full)'
relations:
- reviews: story:seen-documents-modelled
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
The pass was cut short. The root filesystem filled up (0 MB free) before any adversary case was saved, so none ran red. Every finding below comes from reading the code and was not checked by a test, except the ESS probe in finding 4.

```
unit: U2 story:seen-documents-modelled, uncommitted working tree on base 6352006 (cortex-w2-seen)
verdict: NEEDS-CHANGE (by reading, no case run; the pass stopped when / reached 0 MB free)
cases: executed n/a→n/a, red 0
origin: introduced 5 / pre-existing 2 / undecided 0
wrote-outside-worktree: 5 paths, all under ~/.cache/cortex-wave-20261005c/seen/adversary-1/
needs-coordinator: free disk on /, then re-dispatch pass 1 (the case file was lost); decide whether to file an ESS gap for finding 4
```

**1. `git diff --stat`:** could not run, because every Bash call failed with ENOSPC. No file in the tree was changed: the Write of `tests/adversary_seen.rs` failed with "ENOSPC: no space left on device", and Read confirms it does not exist. A `touch` probe may have left a zero-byte `tests/.adv-probe`; the coordinator removed it afterwards.

**2. Cases:** none exist; the write failed. These were meant for `tests/adversary_seen.rs`:
- digest: a seed file renamed inside a seed directory
- digest: bytes moved between two seed files
- `update` of a removed instance whose directory was deleted
- `update` with the frozen spec missing, and with a frozen spec the parser refuses
- a reformatted YAML spec with the same seed → `updated`
- `./docs` vs `docs`
- a changed instructions file → `seed-change-refused`
- a removed instance's name stays taken
- two concurrent `create` of one name
- `cortex list` time with a 512 MiB sparse frozen seed file
- `SeenState::rows` against a state file on disk
- the suite still checks that a refused update leaves the whole instance unchanged (`expect_complete_subject_unchanged`)
- `tests/conformance.rs`: `expect_error` ignores `fields`, and `excludes` passes on a field the view does not show

**3. Suite run:** none, because of the full disk and the missing cases.

**4. Findings** (against the working tree on 6352006)

| # | file:line | what I read | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | src/home.rs:68 | An unreadable frozen spec gives an empty digest. The generated `update_instance` (behaviour.rs:239) checks `seed-change-refused` before `not-active`, so a removed instance whose directory was deleted answers `seed-change-refused`. The base (main.rs@6352006:527-532, `Err(_) => false`) answered `not-active`. | `cortex remove` keeps the directory (limits.md:40); an operator who deletes it to reclaim disk hits this | NEEDS-CHANGE | introduced |
| 2 | src/home.rs:68 | An active instance whose frozen `instance.yaml` is missing or does not parse gets `seed-change-refused`, with detail `{"name"}` and no mention of the frozen spec, and no `update` can recover it. The base updated and rewrote the file. The empty digest is a safe refusal but an unclear one. | a hand edit, or a frozen spec a later cortex refuses to parse | NEEDS-CHANGE | introduced |
| 3 | src/home.rs:90 | The doc comment says the digest covers "the path and bytes of every seed file under `dir`". The code (home.rs:115-127) hashes only the seed root path plus the concatenated bytes, so a file renamed inside the directory, or bytes moved between files, keep the same digest. The base `seed_bytes` (main.rs@6352006:601) had the same blind spot. | any rename inside a seed directory: the update is accepted and the frozen copy stays as it was | CONFIRMED | introduced (the doc claim) |
| 4 | spec/suite.json | Refused updates (`not-active`, `seed-change-refused`) no longer check that the whole instance is unchanged (`expect_complete_subject_unchanged`), and the `Removed/refuses/UpdateInstance` scenario is gone. After a refusal the suite checks only name, seed_digest and state, so a refusal that changed description or model would still pass. ESS forces this: on a scratch copy with the old state guard, `ess specify validate` exited 1 with ESS-COMMAND-004 ("subject fact and lifecycle guards cannot be combined"). | every suite run (the generated code is correct today) | CONFIRMED | introduced |
| 5 | tests/conformance.rs:377 | `expect_error` never compares `fields`, so six scenarios' error contents go unchecked, the new `name-taken` `{"name": "name-1048594"}` among them | every suite run | CONFIRMED | pre-existing (same at @6352006:380) |
| 6 | tests/conformance.rs:448 | the new `excludes` handler passes on a field the rows do not carry, where `contains` would fail on the same field | nothing found: ESS emits only fields the view publishes | INFEASIBLE | introduced |
| 7 | src/home.rs:194 | Every `load_registry` (main.rs:173; every command except schema, setup and mcp-line, including timer-driven `run`) rehashes every instance's frozen seed, removed ones too, under the home lock, and `fs::read` holds each file whole in memory. The base read seeds only on `update`. | any instance with a large seed corpus (not measured) | NEEDS-CHANGE | introduced |
| 8 | src/home.rs:107 | the seed path is hashed as written, so `./docs` vs `docs` counts as a seed change; the base compared `old.seed != spec.seed`, with the same result | an operator who respells the path | CONFIRMED | pre-existing |

Fixes, named only:
- 1–2: `frozen_seed_digest` returns `Result`, and `update` fails with "cannot read <frozen spec>" (or keeps the base behaviour)
- 3: hash each file's relative path and length
- 7: compute the frozen digest only inside `update`

**5. Attacked, could not break (by reading)**
- Name-taken via `existing_instance` matches the old `external` flag: both look up `registry.instances` by exact key.
- Case: `is_name` allows lowercase only, so names cannot differ by case.
- A removed instance stays in the registry, so its name stays taken.
- The home lock is held from load to save, so concurrent `create` runs are serialised.
- Registries written by the base code keep the same format, and `seed_digest` is never written into the registry.
- `SeenState::rows` yields one row per document in the state file, with the fields as written. Only its own test calls it.
- `expect_event` literal comparison matches ESS 0.52.0 `ExpectEvent.payload` (scenario.rs:2129).
- Whitespace-only YAML changes do not move the digest. Whitespace changes inside a seed file or the instructions file are refused, as in base.

**6. Paths written outside the worktree** (all under `~/.cache/cortex-wave-20261005c/seen/adversary-1/`)
- `suite_diff.py`
- `restore_state_guard.py`
- `ess-probe-validate.log`
- `ess-probe-synth.log`
- `spec-probe/` (the coordinator removed it afterwards)

**7. Findings block**

```findings
[
 {"file":"src/home.rs","line":68,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a removed instance whose directory was deleted answers seed-change-refused instead of not-active, because an unreadable frozen spec yields an empty digest and the refusal is checked first (by reading, no case run)"},
 {"file":"src/home.rs","line":68,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"an active instance with a missing or unparseable frozen spec is refused as a seed change with no mention of the file, where the base updated (by reading)"},
 {"file":"src/home.rs","line":90,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the doc comment promises the digest covers every seed file's path, but only the seed root path and the concatenated bytes are hashed, so renames and moved bytes go unseen"},
 {"file":"spec/suite.json","category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"refused updates no longer check that the whole instance is unchanged and the Removed/refuses scenario is gone, a loss forced by ESS-COMMAND-004 with no ESS gap filed"},
 {"file":"tests/conformance.rs","line":377,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"expect_error ignores its fields, so six scenarios' error contents, the new name-taken name among them, are never checked"},
 {"file":"tests/conformance.rs","line":448,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the new excludes handler passes on a field the view does not publish, a state ESS-synthesized suites do not produce"},
 {"file":"src/home.rs","line":194,"category":"judgement","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"every command that loads the registry rehashes every instance's frozen seed, removed ones included, under the home lock and holding each file whole in memory (not measured)"},
 {"file":"src/home.rs","line":107,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"the seed path is hashed as written, so ./docs vs docs is refused as a seed change"}
]
```

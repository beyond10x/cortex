---
format: aep.planning-md/3
id: review-result:adversary-structured-from-files-and-drops-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: structured-from-files-and-drops'
relations:
- reviews: story:structured-from-files-and-drops
revision: 1
---
unit: cortex wave 20261006f unit a, story:structured-from-files-and-drops; uncommitted working tree on base `bc3df39` at ~/.local/state/worktree/trees/b10x/cortex/cortex-w13-a
verdict: NEEDS-CHANGE (1 blocker, 2 warnings)
cases: executed 152→157 (the `structured_files` and lib tests only; the before figure is the same run without the 5 `adv_a_` cases; the full-gate figure is 413), red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (part 6)
needs-coordinator: none (I took no worktree lease)

**1. Diff stat** — this is the same as the implementor's diff. My only edit is new lines 395–551 appended to the untracked `tests/structured_files.rs`. No non-test path changed.
```
 src/ekr.rs | 171 ++-  src/main.rs | 6 +  src/run.rs | 87 ++-  src/sources.rs | 114 ++--
 src/structured.rs | 366 +++--  tests/e2e.rs | 21 +-  tests/structured.rs | 18 +-  website/docs/spec-file.md | 37 ++-
 8 files changed, 734 insertions(+), 86 deletions(-)      ?? tests/structured_files.rs
```

**2. Cases added** (each one run alone first; logs in `~/.cache/cortex-wave-20261006f/a/adv/`)

| case | now | red output |
|---|---|---|
| `adv_a_a_glob_that_matches_no_file_ends_nothing` (:414) | red | exit 0, `"retracted":3`; left `{Ada: [], Grace: [], Linus: []}` right `{Ada: ["Core"], Grace: ["Core"], Linus: ["Ops"]}` |
| `adv_a_a_record_not_applied_as_too_large_keeps_its_values` (:456) | red | `"documents_applied":0,"retracted":1,"skipped":["files:*.json:P-1: its mapped values take 20083 bytes … not applied"]`; Ada `Core` is `Retracted`; left `[]` right `["Core"]` |
| `adv_a_a_model_value_from_a_connectors_source_on_the_same_operation_is_not_ended` (:497) | red | `[Retract { assertion: "email", reason: "dir:people.list:P-1 no longer lists this value (dropped: Supersede)" }]` |
| `adv_a_a_value_that_changes_back_is_active_again` (:432) | green | – |
| `adv_a_a_value_with_surrounding_space_stays_active` (:479) | green | – |

**3. Suite run** (after the cases existed)
- `cargo test --locked --test structured_files` gave EXIT=101: 6 passed, 3 failed (the three red cases above).
- `cargo test --locked --lib` gave EXIT=0: 148 passed.

**4. Findings**

- **F1, blocker, NEEDS-CHANGE / introduced.** If the glob matches no file, every value the source made is retracted.
  - **Measured:** `src/run.rs:475` calls `end_dropped` with an empty `listed`, because `fetch_structured_files` (`src/sources.rs:380`) returns `Ok([])` when nothing matches.
  - **What reaches it:** an export file that is renamed or moved, a directory that is emptied, or a glob that is edited. The default `dropped: Supersede` path needs no further setup.
  - A file holding `{"people": []}` takes the same path; I reasoned this from the code and did not run it.
  - **Fix (I did not apply it):** fail the fetch when no file matches, and/or end nothing when `listed` is empty.
- **F2, warning, NEEDS-CHANGE / introduced.** A record skipped as too large has its old values retracted, and its new value is never written.
  - **Measured:** `src/run.rs:845` does `seen.record` without applying the record, so the record is not in `unsettled` (`src/run.rs:930`).
  - This contradicts `website/docs/spec-file.md:352`: "a record whose current text the run did not apply … keeps its values".
  - **What reaches it:** any record whose mapped values exceed 16,384 bytes.
  - **Fix:** treat skipped keys as unsettled.
- **F3, warning, CONFIRMED / introduced.** A `kind: connectors` (model) source on the same adapter:operation has its values treated as this source's and retracted.
  - **Measured:** both sources key records `<adapter>:<operation>:<id>` (`src/sources.rs:673` and `:715`, `Origin::Record`). `owner` (`src/structured.rs:346`) claims those records, and any property the mapping does not list is retracted.
  - This contradicts spec-file.md:352, "Values another source asserted are never ended". The documented exception covers only *structured* sources that share a prefix.
  - **What reaches it:** the spec accepts both sources together, and I found no validation that refuses it. I did not find any instance that is set up this way.
- **Judgement, not raised as findings:**
  - Two `files` sources with the same `glob` and different `paths` share one prefix (`files:*.json`) and end each other's records. This is documented, but `*.json` is the common glob.
  - Changing a glob changes every identity, which forks nodes and leaves the old values un-ended.
  - The edge-deletion design has a gap I did not test. If more than 1,000 end operations are split into chunks, and the chunk holding the `DeleteEdge` operations fails after the retracts were committed, the edges are never deleted on a rerun. The rerun only looks at active assertions.

**5. Attacked and not broken**
- A value that flips Core→Ops→Core ends Active.
- Surrounding whitespace in a value survives a no-op run.
- A malformed file fails the fetch (the existing test).
- A missing root fails through the walkdir error.
- A prefix collision like `files:a*` vs `files:a` is prevented by the `:` check in `owner`.
- Mixed-evidence assertions are skipped.
- Rejected or held-back records stay unsettled.
- An end transaction that fails gets recomputed from the store on the rerun.
- Redaction is consistent: `listed` is taken after `prepare`.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261006f/a/adv/` (case and suite logs)
- `~/.cache/cortex-wave-20261006f/a/tmp/` (TMPDIR, empty now)
- The assigned build dir `~/.cache/b10x-target/cortex-w13-a` was reused. Disk is at 3.0G free.

**7. Findings block**
```findings
[
  {"file": "src/run.rs", "line": 475, "category": "boundary", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a files glob matching no file yields an empty listing and dropped: Supersede retracts every value the source ever made (test adv_a_a_glob_that_matches_no_file_ends_nothing)"},
  {"file": "src/run.rs", "line": 845, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a record skipped as over the evidence payload bound is marked seen, so end_dropped retracts its old values although the spec says an unapplied record keeps them (test adv_a_a_record_not_applied_as_too_large_keeps_its_values)"},
  {"file": "src/structured.rs", "line": 346, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "owner claims record:<adapter>:<operation>:* evidence that a kind: connectors model source also issues, so its unmapped values are retracted despite the spec saying another source's values are never ended (test adv_a_a_model_value_from_a_connectors_source_on_the_same_operation_is_not_ended)"}
]
```
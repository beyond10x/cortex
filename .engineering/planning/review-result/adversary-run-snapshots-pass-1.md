---
format: aep.planning-md/3
id: review-result:adversary-run-snapshots-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: run-snapshots (wave 20261005h)'
relations:
- reviews: story:run-snapshots
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
unit: story:run-snapshots (U3, wave 20261005h), uncommitted working tree on base dfe3fc1 in ~/.local/state/worktree/trees/b10x/cortex/cortex-w7-snap
verdict: CONFIRMED (6 red cases; 1 warning holds for this unit, 5 are notes or warnings)
cases: executed 256→262, red 6
origin: introduced 9 / pre-existing 0 / undecided 1
wrote-outside-worktree: 2 paths (the scratch dir and the assigned build dir; listed in part 6)
needs-coordinator: whether "seen state is not rewound" is an accepted design (the spec says so at spec/domains/instance.yaml:901) or a blocker. I took no worktree lease.

**1. Diff stat**

`git --no-pager diff --stat` reads `122 files changed, 2177 insertions(+), 324 deletions(-)`. That is exactly what it read before I started, so I changed no tracked file. My only addition is the untracked test file `tests/snapshots_adversary.rs`. No non-test path was touched.

**2. Cases added** (`tests/snapshots_adversary.rs`, headed "ADVERSARY CASES")

Each case was run alone before the suite. All six are red now. The logs are `red-<case>.log` in the scratch dir.

| case | asserts | red output, verbatim |
|---|---|---|
| `adversary_a_restored_store_gets_the_undone_runs_documents_again` | after a restore, the next run applies the undone run's document again | `the document the undone run applied is never applied to the restored store again: {..."documents_applied":0,"documents_new":0...} left: Some(0) right: Some(1)` |
| `adversary_rotation_never_deletes_a_file_cortex_did_not_take` | an operator copy `pinned-before-incident.sqlite` survives 2 more runs at the default keep of 3 | `rotation deleted the operator's file: ["1791224636902-news.sqlite", "1791224638064-news.sqlite", "1791224638889-news.sqlite"]` |
| `adversary_a_snapshot_is_named_by_its_runs_start` | the snapshot's `<time>` is the run's start, as operating.md:102 says | `left: ["1791224639767-news"] right: ["1791224639655-news"]` |
| `adversary_a_run_whose_apply_fails_takes_no_snapshot` | operating.md:103 "A run that applies nothing takes none", tested with ekr refusing the apply | `{"outcome":"apply-refused"} left: ["1791224641076-news", "1791224641704-news"] right: ["1791224641076-news"]` |
| `adversary_rotation_leaves_no_side_files_of_a_restored_snapshot` | after a restore and a rotation, `snapshots/` holds only the kept `.sqlite` | `left: ["1791224644240-news.sqlite-shm", "1791224644240-news.sqlite-wal", "1791224644723-news.sqlite"]` |
| `adversary_a_snapshot_cut_short_leaves_no_copy_behind` | a run killed during the copy (`prlimit --fsize`, SIGXFSZ) leaves nothing behind once a later run succeeds | `left in snapshots/: [".1791224645664-news.sqlite.tmp", ".1791224645664-news.sqlite.tmp-journal"]` |

I also wrote a 7th case, a full disk during the snapshot copy (EFBIG with SIGXFSZ ignored). It was green, so I removed it from the file (see part 5).

**3. Suite run** (after the cases existed)

- **The gate:** `task check ESS=$HOME/.cache/ess/toolchains/0.52.0/ess` passed validate, drift, fmt and clippy, then stopped at the tests:
  ```
  tests/snapshots_adversary.rs ... test result: FAILED. 0 passed; 6 failed
  error: test failed, to rerun pass `-p cortex-cli --test snapshots_adversary`
  task: Failed to run task "check": task: Failed to run task "test": exit status 101
  EXIT=201
  ```
- **Full count:** cargo stops at the first failing binary, so I ran `cargo test --locked --workspace --no-fail-fast` once for the full count. It printed `error: 1 target failed: -p cortex-cli --test snapshots_adversary`, `EXIT=101`, and the result lines sum to passed 256, failed 6, executed 262.
- **Before number:** the 256 comes from the implementor's own `snap/gate.log`, summed from its `test result:` lines.

**4. Findings** (all cover the working tree on dfe3fc1)

| file:line | verdict / origin | finding | what reaches it |
|---|---|---|---|
| src/snapshot.rs:227 | CONFIRMED / introduced, warning | A restore leaves the seen state in place. Documents the undone run applied are never applied again while their text stays the same, so the store is missing them for good. Restoring a snapshot taken before source A's bad run also undoes later good runs of source B, and B's documents are lost the same way. | The documented workflow, "restore the newest snapshot" (operating.md). |
| src/snapshot.rs:52 | CONFIRMED / introduced, warning | `taken_at` gives 0 to any name that does not start with a timestamp. Rotation (:127-130) then deletes such a file first. | Copying a snapshot under its own name inside `snapshots/` is the only way to keep it past rotation and still restore it with `cortex restore`. |
| src/run.rs:509 | CONFIRMED / introduced, warning | The snapshot is taken and rotated before the first apply succeeds. A run whose first apply fails still takes a snapshot, which contradicts operating.md:103. A string of failing runs pushes every snapshot taken before the bad run out of `keep`. The same applies at run.rs:619 for structured sources. | Any `apply-extraction` refusal (outcome `apply-refused`), for example after a bad run. |
| src/snapshot.rs:112 | CONFIRMED / introduced, note | The snapshot is named by the clock at its first apply, not by the run's start, which contradicts operating.md:102. It does not match `runs/<started>-<source>` or the `at` field in cortex.log. | Every run that takes a snapshot. |
| src/snapshot.rs:129 | CONFIRMED / introduced, note | Opening a snapshot read-only (:252) creates `-shm` (32 KB) and `-wal` files next to it. Rotation deletes only the `.sqlite`, so those files stay forever. ekr 0.0.30 stores are in WAL mode (I measured `journal_mode=wal` and header bytes 2/2), so every snapshot is a WAL file. | Every restore. |
| src/snapshot.rs:44 | CONFIRMED / introduced, note | A run killed during the copy leaves `.<name>.sqlite.tmp` (a full copy of the store) and `-journal`. `list` skips hidden files, so nothing ever removes them. | A shutdown, an OOM kill or a unit timeout while the copy runs. The window grows with store size. |
| src/run.rs:61 | CONFIRMED / introduced, note | `Report.snapshot` is set but never shown, and tests/snapshots.rs:86 asserts it is absent. The operator has to list the directory and match times, and the times are not run starts (row above). | Every restore decision. |
| src/snapshot.rs:165 | INFEASIBLE / undecided, note | `holder()` cannot see processes of other users or processes hidden by `hidepid`. Between the check and the copy (:245) there is a race. SQLite's locking keeps SQLite readers consistent. | I could not construct a second user here. |
| src/schedule.rs:182 | CONFIRMED / introduced, note | `restart_view` runs `start` without checking whether the viewer was active, so a restore starts a viewer the operator had stopped. | Only the code: there is no `is-active` call. The fake systemctl cannot show the prior state. |
| src/snapshot.rs:227 | CONFIRMED / introduced, note | A restore takes no snapshot of the current state first. Restoring the wrong snapshot cannot be undone. | `cortex restore` with an older name. |

**5. Attacked and could not break**

- **Path traversal:** `..`, `/`, empty names and hidden tmp names are all stopped by the `list()` membership check.
- **`keep` values:** `keep: 0` and a negative keep both take no snapshot.
- **`busy` under the home lock:** `try_lock` answers `busy` and the registry is not saved.
- **A timer run during a restore:** it blocks on the flock until the restore ends.
- **Disk full during the copy (EFBIG):** the run fails, the head is unchanged, no partial file is left, and the next run applies.
- **Postgres:** refused before the snapshot directory is read, and no snapshot is taken.
- **Seed run:** takes no snapshot, as the code comment says.
- **Store format change:** ekr 0.0.26, 0.0.29 and 0.0.30 seed the same schema (28 tables, `user_version` 0), so there was nothing to break with the installed versions.
- **Viewer on failure paths:** on `busy` and on `Failed` the viewer is started again; if stopping it fails, nothing runs.

**6. Paths written outside the worktree**

- `~/.cache/cortex-wave-20261005h/snap/adversary-1/`: `build.log`, `gate.log`, `suite-nff.log`, 7 `red-*.log` files, `probe/` (scratch ekr stores, host.json, seed yaml, err*.txt), and an empty `tmp/`.
- `~/.cache/b10x-target/cortex-w7-snap`: the assigned build directory, shared with the implementor.

```findings
[
 {
  "file": "src/snapshot.rs",
  "line": 227,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "restore leaves the seen state in place, so the undone run's documents (and later runs' documents from other sources) are never applied to the restored store again while their text stays the same"
 },
 {
  "file": "src/snapshot.rs",
  "line": 52,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a file in snapshots/ whose name does not start with a timestamp sorts as time 0 and rotation deletes it first, including an operator's pinned copy"
 },
 {
  "file": "src/run.rs",
  "line": 509,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a run whose first apply-extraction is refused still takes and rotates a snapshot, contradicting \"a run that applies nothing takes none\" and pushing out snapshots taken before a bad run"
 },
 {
  "file": "src/snapshot.rs",
  "line": 112,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the snapshot is named by the clock at its first apply, not by the run's start that operating.md:102 documents and the runs/ directory and cortex.log use"
 },
 {
  "file": "src/snapshot.rs",
  "line": 129,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a restore leaves -shm and -wal files beside the WAL-mode snapshot, and rotation removes only the .sqlite, so they stay forever"
 },
 {
  "file": "src/snapshot.rs",
  "line": 44,
  "category": "concurrency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a run killed during the copy leaves a hidden .tmp full copy of the store and its -journal, which list skips and nothing ever removes"
 },
 {
  "file": "src/run.rs",
  "line": 61,
  "category": "judgement",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Report.snapshot is never shown in the run output or cortex.log, so the operator must match snapshot files to runs by a time that is not the run's start"
 },
 {
  "file": "src/snapshot.rs",
  "line": 165,
  "category": "concurrency",
  "severity": "note",
  "verdict": "INFEASIBLE",
  "origin": "undecided",
  "message": "holder() cannot see other users' or hidepid-hidden processes and races between the check and the copy; not constructible here"
 },
 {
  "file": "src/schedule.rs",
  "line": 182,
  "category": "judgement",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "restart_view starts the viewer without checking whether it was active, so a restore starts a viewer the operator had stopped"
 },
 {
  "file": "src/snapshot.rs",
  "line": 227,
  "category": "judgement",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a restore takes no snapshot of the current state first, so restoring the wrong snapshot cannot be undone"
 }
]
```
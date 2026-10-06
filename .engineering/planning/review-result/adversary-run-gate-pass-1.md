---
format: aep.planning-md/3
id: review-result:adversary-run-gate-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: run-gate (wave 20261006a)'
relations:
- reviews: story:run-gate
revision: 1
---
```
unit: U1 story:run-gate (wave 20261006a), working tree of cortex-w8-gate on base 88b87fd plus the uncommitted change
verdict: NEEDS-CHANGE
cases: executed 305→311, red 3
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, ~/.cache/cortex-wave-20261006a/gate/adversary-1/ (the assigned scratch)
needs-coordinator: decide whether a failed gate that hits a Busy restore (an attached `ekr mcp`) is a blocker for this unit or gets its own story
```

**1. `git --no-pager diff --stat`** (this is the implementor's diff; I did not change it)
```
 src/lib.rs                |   1 +
 src/run.rs                | 142 +++++++++++++++++++++++++++++++++++++++++-----
 website/docs/operating.md |  11 ++++
 website/docs/spec-file.md |  19 +++++++
 4 files changed, 159 insertions(+), 14 deletions(-)
```
Untracked: `src/gate.rs` and `tests/run_gate.rs` are the implementor's. The only file I added is `tests/run_gate_adversary.rs`, headed "ADVERSARY CASES". I changed no non-test file.

**2. Cases** (each run alone first; logs in `adversary-1/red-<case>.log`)

| case | asserts | now |
|---|---|---|
| `adversary_a_failed_run_is_undone_while_an_mcp_reader_is_attached` | starts `ekr mcp` exactly as `cortex mcp-line t` prints it, then a run fails `facts_refused max 0`; the head should be back at the pre-run revision | RED |
| `adversary_keep_1_the_snapshot_the_log_names_still_exists` | with `snapshots: {keep: 1}`, the `gate.restored` in the log should still exist | RED |
| `adversary_a_cost_equal_to_its_max_passes` | cost 0.1+0.2 summed through `add_cost` against `max: "0.3"` should pass | RED |
| `adversary_a_late_batch_failing_the_gate_undoes_every_batch` | 30 docs in 3 batches; the third pushes `facts_refused` to 3 (max 2); store and state are undone, `pending_since` is set | green |
| `adversary_a_measure_equal_to_its_min_or_max_passes` | a measure equal to its min or its max passes | green (catches the mutant, see below) |
| `adversary_a_structured_run_over_its_gate_is_undone` | a structured source with 3 records against `documents_applied max 1` is undone | green |

The structured case first failed because my own test spec had no `aliases` line. That was my mistake, not a finding; I fixed it and it ran green.

Red output, verbatim:
```
panicked at tests/run_gate_adversary.rs:158:5:
assertion `left == right` failed: a run that failed its gate was left in the store while an MCP reader was attached: {"command":"run","detail":{"reason":"the run failed its gate: facts_refused = 1 (max 0); the run was not undone: process 2391035 holds ~/.cache/cortex-wave-20261006a/gate/adversary-1/tmp/cortex-e2eie2L6w~/t/store.sqlite open"},"outcome":"apply-refused"}
  left: 3
 right: 0
---
panicked at tests/run_gate_adversary.rs:183:5:
the log names snapshot "1791244852470-news", the instance holds ["1791244852952-before-restore"]
---
panicked at tests/run_gate_adversary.rs:254:5:
a run costing 0.3 USD failed max 0.3: ["cost_usd = 0.30000000000000004 (max 0.3)"]
```

**Mutant probe.** I copied `src/gate.rs` to `adversary-1/mutant-min/` and changed `v < lo` to `v <= lo`. The implementor's 5 unit tests stayed green (`test result: ok. 5 passed`). My equal-bounds case went red: `left: [Failed { measure: "documents_applied", value: Some(1.0), min: Some("1") ...}] right: []`. The e2e test `a_run_within_its_gate_is_kept` uses 2 applied against min 1, so I expect it would not catch the mutant either. I did not run that one against the mutant.

**3. Suite**, run after the cases existed: `task check ESS=$HOME/.cache/ess/toolchains/0.52.0/ess` writing to `adversary-1/gate.log`
```
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.69s
error: test failed, to rerun pass `-p cortex-cli --test run_gate_adversary`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```
`task check` stops at the first failing test binary. So the 311 executed (308 passed, 3 failed) comes from `cargo test --locked --workspace --no-fail-fast` (`adversary-1/suite-nofailfast.log`, EXIT=101). The 305 before is from the implementor's `gate.log`. The first `task check` failed `cargo fmt --check` on my own file; I ran `rustfmt` on that one file only.

**4. Findings**

| file:line | verdict | origin | what was measured / what reaches it |
|---|---|---|---|
| src/run.rs:640 | NEEDS-CHANGE | introduced | **Measured:** red case at :158. **Reached by:** `cortex mcp-line` sets up `ekr mcp`, and a live probe showed it holds `store.sqlite` and its `-wal`/`-shm` open from startup. The restore is refused as Busy, so the run is not undone, but `state/` keeps the documents as seen. The next run therefore has nothing new, reports `ran` and resets the failure count, and the bad data stays. `operating.md:101` says "On a `sqlite` store the run is undone first" with no exception. |
| src/run.rs:638 | CONFIRMED | introduced | **Measured:** red case at :183. With `keep: 1`, `restore_held` publishes `<now>-before-restore` and then rotates away the run's snapshot, so `gate.restored` names a snapshot that is gone. **Reached by:** `snapshots: {keep: 1}` in a spec. The undo itself works. |
| src/gate.rs:166 | CONFIRMED | introduced | **Measured:** red case at :254. Bounds and the cost are compared as f64, even though the spec carries decimals as strings and cortex prints the cost at `{:.4}`. **Reached by:** a run whose cost lands exactly on a max (needs real answer costs whose float sum overshoots). |
| src/gate.rs:174 | CONFIRMED | introduced | **Measured:** the mutant probe above. No test in the unit pins the case where a measure equals its min. |

**5. Attacked and could not break**
- **A late batch failing the gate:** every batch is undone, and `pending_since` is set (green case).
- **Partial counts:** the gate runs once, after the loop, including runs stopped by `stopped`.
- **Window and `held_since`:** `finish` is not called on a gate failure, and `state/` (with `entities.json`) is rewound.
- **Structured path:** green case.
- **Decimal bounds:** `NaN`, `inf` and `1e3` are refused by the `spec.rs` regex.
- **`cost_usd` null:** fails its check, as documented.
- **Paths into arrays:** they cannot be read and so fail the check. The only array `ekr quality` prints is `shared_names`, so this is low impact.
- **`record-failure` counting a gate failure, so two bad runs disable the source:** intended by the story's Work section and the docs. Not a finding.
- **keep 0 and postgres:** documented as "not undone".
- **Concurrent timer runs:** serialised by `home.lock()` (main.rs:186).
- **`ekr quality` slow:** not tested. It has no timeout and holds the home lock while it runs.

**6. Paths written outside the worktree**, all under `~/.cache/cortex-wave-20261006a/gate/adversary-1/`:
- `build.log`, `gate.log`, `suite-nofailfast.log`, `mutant-min.log`
- six `red-*.log`
- `mutant-min/` (Cargo.toml, src/lib.rs, tests/eq.rs)
- `tmp/` (test tempdirs)
- `probe/` held a copy of a local instance store for the `ekr mcp` probe. I deleted it.

I built only into `~/.cache/b10x-target/cortex-w8-gate`.

```findings
[
  {"file": "src/run.rs", "line": 640, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "While an ekr mcp server from cortex mcp-line holds the store open, the restore is refused as Busy, so a gate-failed run stays in the store with its documents marked seen and is never retried, contrary to operating.md:101."},
  {"file": "src/run.rs", "line": 638, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "With snapshots.keep 1, the before-restore snapshot rotates away the run's snapshot, so cortex.log gate.restored names a snapshot that no longer exists."},
  {"file": "src/gate.rs", "line": 166, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Decimal bounds and the float-summed cost are compared as f64, so a run costing 0.1+0.2 USD fails max \"0.3\" as cost_usd = 0.30000000000000004."},
  {"file": "src/gate.rs", "line": 174, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Mutating v < lo to v <= lo leaves the unit's 5 gate tests green; only the added equal-bounds case catches it."}
]
```
---
format: aep.planning-md/3
id: review-result:security-w30a-tail-pass-1
kind: review-result
status: active
title: 'Security review pass 1: unit tail (wave 20261030a)'
relations:
- reviews: story:connectors-call-deadline
- reviews: story:window-relative-placeholders
- reviews: story:runs-retention
- reviews: story:small-robustness
revision: 1
---
unit: cortex wave 20261030a, unit `tail`, branch `unit/w30a-tail`. Findings cover the unit at 4f30cb2d (base effef82c) plus my test commit 0b554f9c.
verdict: NEEDS-CHANGE
cases: executed 342→349, red 4
origin: introduced 2 / pre-existing 3 / undecided 3
wrote-outside-worktree: 2 scratch files, plus test temp dirs and a lease record (part 6)
needs-coordinator: origin of findings 1, 2 and 4. I had no base worktree, so I could not run the cases there. Reading `git show effef82c:` says they fail at base too, because base has no masking, no deadline and no cap at all. Findings 1 and 2 still fall inside this unit's acceptance statements.

**1. Diff stat** (my commit 0b554f9c, made through `b10x-gates bot` as `b10x-bot[bot]`, author and committer):
```
 tests/w30a_tail_security.rs | 401 ++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 401 insertions(+)
```
It touches only a test file. `tests/w30a_tail_adversary.rs` is the adversary's untracked file. I did not stage or touch it.

**2. Cases added** (`tests/w30a_tail_security.rs`), each run alone first:

| case | asserts | now |
|---|---|---|
| `a_credential_cut_by_the_session_stderr_window_is_not_kept_unmasked` | an ekr session ends after writing a password assignment and then enough output that the 16 KiB window starts inside the name; the value must appear in no printed line and no file | red |
| `pruning_follows_no_symlink_out_of_the_runs_directory` | `runs/<old>-news` is a symlink to an outside dir, and an old run dir holds a symlink outside; the old dir is pruned and both outside targets survive | green |
| `an_inherited_ekr_store_host_backend_or_stage_does_not_redirect_the_run` | with `EKR_STORE`/`EKR_HOST`/`EKR_BACKEND` naming another store and an invalid `EKR_STAGE`, the run applies to its own store and the other store is never created | green |
| `a_connectors_process_left_holding_the_output_does_not_hold_the_call_past_its_deadline` | `invoke` answers and exits but leaves `sleep 10 &` holding stdout; with a 1 s deadline the run must end within 4 s | red |
| `an_out_of_range_deadline_setting_does_not_panic_the_run` | `CORTEX_CONNECTORS_TIMEOUT_S=9223372036854775807` must not panic | red |
| `the_cap_holds_for_a_lookup_reached_through_a_symlink` | a lookup that is a symlink to a file of 16 MiB + 1 byte fails on the cap | green |
| `a_lookup_that_is_a_fifo_fails_the_fetch_instead_of_blocking_the_run` | a lookup that is a FIFO with no writer must fail the fetch within 15 s | red |

Output of each red case run alone (paths with `~`):
- Case 1. The first run failed at the first of its two assertions: `the credential's value is in what the run printed`. I then merged the stdout check and the file check into one assertion so it names both sinks, and reran:
  `the credential's value is kept unmasked: printed in 1 line(s) (first starts Some("{\"command\":\"run\",\"detail\":{\"reason\":\"ekr schema failed: the run's ekr session ended (it closed its output): ekr session ended with exit status: 1: rd=hunter2Qz7vX9wK\\n")), written to ["~/.cache/claude-tmp/cortex-e2e350vf3/home/instances/m/status/news.json"]`
- Case 4: `the call with a 1 s deadline held the run 10.071691882s (the left process holds its output 10 s)`
- Case 5: `cortex panicked: thread 'main' (1823717) panicked at src/terminate.rs:72:20: overflow when adding duration to instant`
- Case 7: `the run was still blocked after 15.035378992s on a lookup that is a FIFO`

**3. Suite run** (after the cases existed):
`CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test --lib --test w30a_tail_security --test small_robustness --test runs_retention --test connectors_call_deadline --test file_records --test structured_files` gave EXIT=101.
- Passing: lib 303, connectors_call_deadline 1, file_records 18, runs_retention 3, small_robustness 4, structured_files 13.
- w30a_tail_security: 3 passed, 4 failed (cases 1, 4, 5 and 7).
- The `before` count of 342 is this same run with my file left out.

**4. Findings** (tree 0b554f9c)

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | src/ekr.rs:604, :939 | The drain keeps the last 16 KiB of session stderr. `Pipe::finish` masks only that kept window, so a credential whose name falls before the cut is kept unmasked. It reached the run's printed answer and `status/news.json` (case 1). Fix: mask before cutting, as `extract::error_text` already does, e.g. cut at a line boundary after masking. | Every run whose session ends goes through `finish` via `Session::request`, which is the default path. Stderr over 16 KiB I constructed; I found no ekr output that large. | NEEDS-CHANGE | undecided |
| 2 | src/terminate.rs:139, :168 | The deadline is checked only while the direct child runs. Once it exits, the stdout and stderr readers are joined with no time limit and the process group is never killed. A process connectors leaves behind held the run for 10 s against a 1 s deadline (case 4). | The story acceptance says a hung call ends within the deadline. Connectors does spawn helpers (`adapters/kubernetes/src/exec_credential.rs`), but I did not show one that inherits cortex's pipes. | INFEASIBLE | undecided |
| 3 | src/terminate.rs:72 | `Instant::now() + limit` panics when the setting is 2^63 or more. The panic comes after spawn, so the child is left unreaped (case 5). Fix: `checked_add`, or clamp the setting. | Only an operator setting that large; nobody was shown to do it. | INFEASIBLE | introduced |
| 4 | src/sources.rs:398 | `read_capped` opens the path without checking its file type. A FIFO lookup blocks the run in `open`, and SIGTERM does not reach that point (case 7). The walkers skip non-regular files, so only `records.lookup` can hit this. | The lookup path comes from the spec; nobody was shown to name a FIFO. | INFEASIBLE | undecided |
| 5 | src/quality.rs:481, :513; src/gate.rs:124; src/instance.rs:408 | ekr stderr from `sample`, `fact-quality`, `quality` (the schema-apply gate, `schema.rs:1460`) and `snapshot` (adopt, `main.rs:1364`) goes into error answers unmasked. No test written. | `cortex quality`, `cortex schema apply` with a gate, `cortex adopt`. None of these files is in the unit's diff. | CONFIRMED | pre-existing |
| 6 | src/ask/records/postgres.rs:481 | The connectors launch's stderr is inherited into the ask service's stderr, bypassing `serve::log`'s mask. No test written. | An ask service with postgres answer records. | CONFIRMED | pre-existing |
| 7 | src/ask/mcp.rs:42 | The MCP reader (`ekr mcp`, direct or launched) does not strip inherited `EKR_*`. It sets HOST/BACKEND/STORE itself (`ask/mod.rs:158-162`), so nothing redirects to another store, but an inherited `EKR_STAGE` or `EKR_FULL_REPLAY` does reach it. No test written. | The ask service started from a shell that carries those variables. | CONFIRMED | pre-existing |
| 8 | src/run.rs:651 | Pruning decides by name only: any directory under `runs/` matching `<digits>-<[a-z0-9-]+>` that is old enough is removed, including one an operator placed there. Cortex cannot tell which directories it created. | Only an operator putting their own directory under `runs/`. | CONFIRMED | introduced |

**Per invariant**

| invariant | status | evidence |
|---|---|---|
| 1. No credential in logs, reports or the model | broken | Case 1 red: the session stderr is cut, then masked (finding 1). One-shot `finished`, `seeded` and session answers are masked whole before any formatting. Connectors errors carry only the envelope's code and stage, and serde errors carry no content. Gaps outside the diff: findings 5 and 6. |
| 2. Pruning stays inside `runs/` | holds (name-only rule, finding 8) | Case 2 green. `DirEntry::file_type` does not follow symlinks. The std `remove_dir_all` (rustc 1.99.1) lstats the top and opens with `O_NOFOLLOW`, so a symlink swapped in after the check is unlinked, not followed. Paths come from `read_dir` of `layout.runs()`, never from file contents. The current run is protected because `begun` is taken before its directory is named. |
| 3. Inherited `EKR_*` cannot redirect the store | holds | Case 3 green. Spawns: `store_cmd` direct (ekr.rs:394) strips all and sets the three, and `cmd` sets or removes STAGE. `Launch::command` (:318) strips. `schema_cmd` (:537) strips and removes the three. quality/gate/instance go through `Store::command`. Not stripped but opening no store: `Binary::host_json` (:146), `model_schema` (:182), `fact-quality` (quality.rs:494). Not stripped but sets the three: the MCP reader (finding 7). `eval.rs:276`, `main.rs:3184` and `schedule.rs:627` write configs that set all three. |
| 4. The deadline kills what it spawned; no credential left behind | broken (findings 2, 3) | The unit's own test plus code: `kill` sends SIGKILL to the process group (`process_group(0)`) and reaps. Input goes over a pipe, with no temp file. On timeout the readers' buffers are dropped and nothing is logged. |
| 5. The 16 MiB cap holds for every file a source reads | holds for size; broken for FIFOs (finding 4) | Case 6 green. `take(MAX+1)` bounds `/dev/zero` and symlinks. The walkers skip symlinks and special files. No other source file read bypasses `read_capped`. |

**5. Checked and found sound:** masking order in `finished`, `seeded` and `Session::request`; the `OUTCOME_UNKNOWN` text survives masking; the `host_json` object checks; the `bare_tag` path is masked before it is reported; `run_started` overflow returns None; the `{since_minutes_ago}` arithmetic saturates; `keep_days < 1` is refused; the structured identity-pair check.

**6. Written outside the worktree** (`~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/tail/`):
- `secrev-suite.txt` (suite log)
- `secrev-msg.P3DrA3` (commit message)
- Test temp dirs `~/.cache/claude-tmp/cortex-e2e*`, removed when the tests ended.
- A worktree session lease `secrev-w30a-tail-125ab618`, acquired and released.

```findings
- file: src/ekr.rs
  line: 939
  category: integrity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: the session's stderr is cut to its last 16 KiB before masking, so a credential whose name falls before the cut is kept unmasked in the run's answer and status file
- file: src/terminate.rs
  line: 139
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: undecided
  message: the connectors deadline stops applying once the direct child exits, so a process it left holding the output pipes holds the call unbounded and is never killed
- file: src/terminate.rs
  line: 72
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a CORTEX_CONNECTORS_TIMEOUT_S of 2^63 or more overflows Instant addition and panics after the connectors child was spawned
- file: src/sources.rs
  line: 398
  category: integrity
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: read_capped opens a records lookup without a file-type check, so a FIFO lookup blocks the run in open where neither the cap nor SIGTERM applies
- file: src/quality.rs
  line: 481
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: ekr stderr from sample, fact-quality (quality.rs:513), quality (gate.rs:124) and snapshot (instance.rs:408) goes into error answers unmasked
- file: src/ask/records/postgres.rs
  line: 481
  category: integrity
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the connectors launch's stderr is inherited into the ask service's stderr, bypassing the masked log
- file: src/ask/mcp.rs
  line: 42
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the MCP reader strips no inherited EKR_* variable, so EKR_STAGE or EKR_FULL_REPLAY from the environment reach ekr mcp, though HOST, BACKEND and STORE are set explicitly
- file: src/run.rs
  line: 651
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: pruning decides by name only and removes any old directory under runs/ named digits-dash-lowercase, including one an operator placed there
```

---
format: aep.planning-md/3
id: review-result:adversary-w30a-ess-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit ess (wave 20261030a)'
relations:
- reviews: story:ess-pin-0-77
- reviews: story:spec-compat-gate
revision: 1
---
unit: `ess` of cortex wave 20261030a, branch `unit/w30a-ess` at `d2d2f378` (base `effef82c`)
verdict: CONFIRMED (3 warnings, no blocker)
cases: executed n/a→n/a, red 0 (I added no Rust case and ran no suite; all three findings are Taskfile or document behaviour, shown by the commands below)
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/ess/adv1/`
needs-coordinator: when `unit/w30a-tail` merges into the wave, `spec/compat-ack.json` must be rewritten (finding 3)

**1. `git --no-pager diff --stat` of the worktree:** empty. `git status --short` shows 0 lines. I changed no file in the worktree and made no commit.

**2. Cases added:** none. `tests/w30a_ess_adversary.rs` does not exist. Findings 1 and 3 are Taskfile behaviour. Finding 2 compares what two documents claim with the source at the release tag; a Rust test for it would only grep old source, so the evidence is commands instead.

**3. Suite run:** none, since I added no case.

**4. Findings** (they cover `d2d2f378`)

1. **`AGENTS.md:59`, `:64`, `:70` — the stated exit codes are wrong.** CONFIRMED, introduced, warning.
   - **What was measured:** AGENTS.md says `task compat` "exits 4" on an unacknowledged change and refuses a stale file "with exit 1". It actually exits 201 in both cases; go-task wraps the command's status. In a scratch clone with one acknowledgement removed:
     ```
     TMPDIR=<scratch>/tmp task compat     -> exit 201   ("task: Failed to run task "compat": exit status 4")
     TMPDIR=<scratch>/tmp task -x compat  -> exit 4
     ```
     The stale-digest cases (a pre-release tag or a stray `v2-experiment` tag on HEAD) also exit 201, not 1.
   - **What reaches it:** any agent or script that follows AGENTS.md and branches on 4 versus 1. The story's own acceptance ("exits non-zero") holds.
   - **Fix:** say that `ess` exits 4 or 1 and `task` reports 201 (`task -x` passes the code through), or call `task -x compat` from `check`.

2. **`spec/compat-ack.md:27-30, :34, :36` and `CHANGELOG.md:69-71` — reasons that are false against the release they compare with.** CONFIRMED, introduced, warning.
   - **The claims:** the rows say the record routes "answered 413 and 400 `bad-request` already … No behaviour changed" and answered 500 `store-failed` "already". They also say `facts_reapplied` is a field "which `cortex run` printed already". The CHANGELOG's Changed bullet repeats the `cortex run` claim. compat-ack.md names the comparison as `v0.6.1`, and at `v0.6.1` none of this existed:
     ```
     git show 'v0.6.1:src/main.rs' | grep -c facts_reapplied                          -> 0
     git show 'v0.6.1:src/ask/serve.rs' | grep -c 'RECORD_BODY_LIMIT\|64 \* 1024'      -> 0
     git show 'v0.6.1:src/ask/serve.rs' | grep -c request_id                           -> 0
     git tag --contains 261ee648 | wc -l    (the commit that added facts_reapplied)    -> 0
     ```
     At `v0.6.1`, `store_failed` sent the store's error text as `message`, with no `request_id`. The CHANGELOG itself lists these as Unreleased changes (`CHANGELOG.md:176-178`, `:200-207`).
   - **What reaches it:** each acknowledgement rests on a reason that `task compat` presents as reviewed. The 1.0 notes then tell readers "printed already" and "no behaviour changed" for changes 1.0 makes. The claims are only true against the wave base `effef82c`.
   - **Fix:** reword these 6 rows and the Changed bullet to say what changed since 0.6.1: the 64 KiB 413 limit is new, and the 500 body now has a fixed message and `request_id`.

3. **`spec/compat-ack.json:4` (`after`) — merging the sibling unit `tail` turns `task compat` red.** CONFIRMED, introduced, warning, category concurrency.
   - **What was measured:** `unit/w30a-tail` adds `InstanceSpec.runs` and the type `RunsPolicy` in `spec/domains/instance.yaml`. A trial `git merge --no-commit origin/unit/w30a-tail` in a scratch clone (aborted afterwards) gives:
     ```
     task compat exit 201
     refused: spec/compat-ack.json: [conflicting_declaration] acknowledgements.after: the acknowledgements are for `after` 454ae2cd…, and this comparison's `after` is 4ead3fc7…
     ```
     The diff also contains a new, unacknowledged `type/cortex.instance.InstanceSpec/field-added/runs` ("unknown for callers").
   - **What reaches it:** the wave's integration merge followed by the wave gate.
   - **What it needs:** after `tail` merges, rewrite `after`, add that id to `compat-ack.json` and add its reason row to `compat-ack.md`. Any later spec-touching unit hits the same thing on the `after` line.

**5. Attacked and could not break**
- **CI sha256:** `dd31b828…0089` is the `SHA256SUMS` entry for `ess-0.77.0-x86_64-unknown-linux-gnu.tar.gz`, the asset the workflow downloads (I fetched it read-only with `gh release download`). The install step checks the line with `grep -qxF` and the file with `sha256sum --check --strict`.
- **`facts_reapplied`:** `SourceRan` and the `ran` detail of `cortex run` both read the same `Report`, on the normal run path (`ports.rs:736`) and the seed-source path (`ports.rs:509`). Undone and failed runs produce no `SourceRan`. A run with no re-applies gives 0 in both; the `cortex.log` line omits a 0 by design. The `cortex create` seed detail lacks the field, but that is already the draft story `create-seed-reports-facts-reapplied`.
- **Ask outcomes against `ask-service.md:193-195`:** the routes answer 413 or 400 `{"error":"bad-request","message":…}` and 500 `store-failed` with `request_id`, as the spec now says. The pre-existing tests `answer_records.rs:791` and `:850ff` hold this.
- **Is conformance exercising them or only accepting them?** Only accepting. The 4 new scenarios force the `external` branch and run only generated code plus the harness mapping. In production the new arms in `records.rs` and `serve.rs` are never reached, since the context never forces those branches.
- **`task compat` edge cases:**
  - No tags, a shallow clone, or a `git archive` that produces nothing: all fail loudly. `git describe` fails under `set -e`, and GNU tar exits 2 on empty input.
  - A wrong, pre-release or stray `v[0-9]*` tag: exit 201, because the acknowledgements are bound to the digest of both sides. There is no silent pass.
  - Comment-only spec edits and edits to `spec/compat-ack.md` or `spec/suite.json` leave the digest unchanged.
  - The temp directory is created under `${TMPDIR:-$HOME/.cache}`, never under `/tmp` unless `TMPDIR` points there, and the trap removed it (0 left after 9 runs).
- **Enum renames:** every renamed variant keeps its `#[serde(rename)]` spelling, so no wire spelling or JSON field changed. Schema diffs are provenance digests only. `.ess-output` is untracked.
- **Generated `Cargo.toml` change:** spec-types moved from `=` to `^` serde pins. This is harmless here: the crate is `publish = false` and builds use `--locked`.
- **Tests:** none were relaxed. The only change to an existing assertion is the scenario floor going from 120 to 124, an exact equality.

**6. Paths written outside the worktree:** `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/ess/adv1/`, which holds:
- `sums/SHA256SUMS`
- `tmp/` (empty)
- `merge.log`
- 8 `out-*.log` files

I deleted the scratch clone and the probe script after use.

```findings
- file: AGENTS.md
  line: 59
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: AGENTS.md says task compat exits 4 on an unacknowledged change and 1 on a stale file, but it exits 201 in both cases (only task -x passes 4 through)
- file: spec/compat-ack.md
  line: 27
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: acknowledgement reasons (rows 27-30, 34, 36) and CHANGELOG.md:69-71 say the 413 limit, the request_id store-failed body and facts_reapplied in cortex run existed already, but none were in v0.6.1, the release task compat compares against
- file: spec/compat-ack.json
  line: 4
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: merging sibling unit/w30a-tail makes task compat refuse with a stale after digest, and its InstanceSpec.runs field is an unacknowledged unknown change
```

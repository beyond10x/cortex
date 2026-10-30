---
format: aep.planning-md/3
id: review-result:hardening-w30a-pass-1
kind: review-result
status: active
title: 'ESS hardening pass 1: techniques 6, 1, 4 (wave 20261030a)'
relations:
- reviews: story:ess-pin-0-77
- reviews: story:spec-compat-gate
revision: 1
---
unit: ESS hardening of cortex spec/ and its Rust runner, tree cortex-w30a-harden, branch unit/w30a-harden, base dba0b9ed, findings cover 53170a27 (spec/ identical to dba0b9ed)
verdict: CONFIRMED (2 warnings, 7 notes; no blocker)
cases: executed 124→124, red 0 (no scenario added; the 124 are the default `spec/suite.json` run after the hook commit)
origin: introduced 0 / pre-existing 9 / undecided 0
wrote-outside-worktree: 1 directory, ~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/harden/ (listed in part 6)
needs-coordinator: the precedence of UpdateInstance on a removed instance with a changed seed (finding 7), and whether the answer records and closed reviews get a view (findings 1 and 2); both are spec-surface decisions, not fixes for this pass

## 1. Diff

```
$ git --no-pager diff --stat dba0b9ed..HEAD
 tests/conformance.rs | 94 ++++++++++++++++++++++++++++++++++++++++++++++++----
 1 file changed, 88 insertions(+), 6 deletions(-)
$ git --no-pager diff --stat
(empty)
```

The only path is a test file. One commit, `53170a27` (author and committer `b10x-bot[bot]`), made
through `b10x-gates bot --repository beyond10x/cortex -- commit -F <file>` after `b10x-gates
scan-text` exited 0 ("1 file(s) carry no private rule"). Not pushed.

The commit is the test-only hook the brief asked for. `CORTEX_CONFORMANCE_SUITE` points
`every_scenario_of_the_synthesized_suite_holds` at another `suite.json`, and the 124-scenario floor
applies only to `spec/suite.json`. `CORTEX_CONFORMANCE_REPORT` writes an
`ess-conformance-report/2` (passed and failed ids; the suite is bound by the SHA-256 of its bytes)
that `--collect` accepts. Unset, the test behaves as before. Checks after the change:
`cargo test -p cortex-cli --test conformance` gave 3 passed (`every_scenario…` ran 124 scenarios),
`cargo fmt -p cortex-cli -- --check` exited 0, and `cargo clippy --locked -p cortex-cli --test
conformance -- -D warnings` exited 0. The full `task check` was not run (a wave gate).

## 2. Cases added

None. Every survivor below needs either a new view or event field (a change to the spec surface) or
a clock seam in `src/ports.rs` (an implementation change). An authored scenario cannot kill a
mutant in `--emit` mode (its expectations are its author's). So nothing within this pass's remit
would have turned a survivor red.

## 3. Techniques run

Order run: 6, then 1, then 4. Not run:
- 2 (random sequences): no Rust explorer exists. It would need the Go or TypeScript package
  `synthesize` writes, plus a Target adapter over cortex's ports. That is a new harness, and its
  cost exceeds this pass.
- 3 (caller replay): no recorded caller stream exists.
- 5 (metamorphic relations): it needs technique 2's walker.
- 7 and 8: already done in this wave (`task compat`; `review-result:design-review-w30a-spec-pass-1`).

### Technique 6: guard analysis

Command: `ess specify compile --path spec --format json --out <scratch>/ir.json`, then a throwaway
probe (Python, outside every repository, deleted after use per the workspace rule) that reads the IR
only. For each command it:
- builds a boundary domain over every input or subject field a `when`, `when_subject`,
  `subject_state` or `external_when` guard reads. Booleans take both values; optionals add absent;
  integers take every guard and invariant literal ±1 plus -1, 0, 1, 2, i64 min and i64 max; enums
  take every lifecycle state; strings take two symbolic values; a row is present or absent.
- drops stored rows that break an entity invariant, because those are unreachable.
- evaluates every outcome in declaration order, which is the precedence `generated/cortex-model`
  implements (checked in `behaviour.rs` for UpdateInstance and ProposeSchemaChanges).
- applies `wrong_state` after a selected move.
- reports four classes: dead guards, guards shadowed by an earlier one, overlaps, and gaps.
- checks each single-field entity invariant after the chosen outcome's `sets`, with the written
  value drawn from its source's domain (an input's type, or the stored value plus the increment).

Scope: 22 commands, 8 input or subject guards, 123 combinations. Externals are decided by the
context, so they are not input guards. An `external_when` was checked only for deadness.

**Planted defects**, each in a copy of `spec/` under scratch. `ess specify validate` accepted both
copies ("4 file(s), valid"), so the compiler does not cover either class:

| plant | edit | the analysis said |
|---|---|---|
| dead guard | `RecordFailure/disabled` `consecutive_failures >= 1` → `consecutive_failures < 0` (dead under the invariant `consecutive_failures >= 0`) | `dead \| cortex.instance.RecordFailure \| disabled \| no input in the boundary domain satisfies the guard` |
| shadowed guard | `ProposeSchemaChanges/reviewed` `when: review` → `when: dry_run` | `shadowed \| cortex.instance.ProposeSchemaChanges \| reviewed \| satisfied only where an earlier outcome answers` |

**Restored run** on the real `spec/`. No dead, shadowed or gap findings. 8 findings in three groups:
- `overlap | UpdateInstance | seed-change-refused & not-active | 4 combinations, e.g. seed_digest differs, state Removed, answered seed-change-refused` gives finding 7.
- `overlap | ProposeSchemaChanges | proposed & reviewed | dry_run true, review true, answered proposed` gives finding 8.
- `invariant-break | RecordAnswer | recorded` ×6 gives finding 6. The six are `latency_ms`,
  `input_tokens`, `output_tokens`, `cached_tokens`, `context_tokens` and `steps` `>= 0`, each false
  for a negative input.

Also checked and holding: `consecutive_failures` and `runs` cannot go negative (`+1` or literal
`0`). A Disabled source always has `consecutive_failures >= 2`, so `RecordFailure` on it always
answers `already-disabled`. The four `RunSource` branches over `fetch_only` (absent, false, true) ×
state leave no input without an answer.

### Technique 1: mutation audit

The repository implements both components (`cortex` and `cortex-ask`) and the runner covers both
domains, so the audit ran without `--component`.

1. `ess verify conform mutate --path spec --emit <scratch>/mut` took 52 s and exited 0. It wrote
   165 mutants, 110 of them stillborn, 6 with synthesis refusals, and 23 unavailable `emit-swap`
   sites.
2. The runner built once (`cargo test -p cortex-cli --test conformance --no-run`), then the test
   binary ran over each suite through `probe/run-mutants.sh`.
3. **Plant before the full run:**
   - The baseline gave `rc=0 passed=124 failed=0`.
   - Mutant `emit-swap/cortex.instance.RemoveInstance/removed/…InstanceRemoved/…InstanceUpdated`
     was expected killed. It gave `rc=101 passed=122 failed=2`, with the failures
     `cortex.instance.RemoveInstance/outcome/removed: cortex.instance.InstanceUpdated was not
     published` and the transition scenario likewise.
   - So the harness applies mutants, and a kill is an assertion failure, not an unsupported step.
4. Each suite took about 0.6 s, so the full run was cheap. All 55 suites with a mutant ran; there
   was no sampling.
5. `ess verify conform mutate --collect <scratch>/mut --report-out <scratch>/mutation-report.json`
   exited 1 (a survivor) and wrote `ess-mutation-report/7`.

**Result: 165 mutants, 39 killed, 11 survived, 5 unwitnessed, 0 equivalent, 0 inconclusive, 110
stillborn, 23 unavailable sites.**

| class | killed | survived | unwitnessed | stillborn |
|---|---|---|---|---|
| emit-swap | 6 | | | |
| guard-boundary | 5 | | | |
| guard-negate | 8 | | | |
| guard-connective | 1 | | 2 | |
| precedence-swap | 1 | | | |
| sets-drop | 2 | | 1 | |
| sets-retarget | 16 | 10 | 2 | |
| order-flip | | 1 | | |
| emit-drop | | | | 29 (ESS-COMMAND-007) |
| error-swap | | | | 77 (ESS-COMMAND-001 ×60, ESS-COMMAND-004 ×17) |
| transition-to | | | | 4 (ESS-ENTITY-011) |

Survivors:
- `sets-retarget` ×8 on `RecordAnswer/recorded` (`answer`, `reason` and the six counters, `latency_ms` among them) give
  finding 1.
- `sets-retarget` ×2 on `CloseReview/closed` (`expect_facts` and `expect_nodes`) give finding 2.
- `order-flip` on `OpenReviews` (`opened_at`) gives finding 3.

Unwitnessed:
- `sets-drop CastVote/changed/value` gives finding 4.
- `sets-retarget RecordAnswer/recorded/{identity,question}` is part of finding 1.
- `guard-connective RunSource/{fetched,fetch-only-failed}` gives finding 5.

Every one of the 77 `error-swap` mutants was stillborn (finding 9).

The killed guard-boundary and guard-negate mutants show that the RecordFailure `>= 1` boundary,
every `fetch_only` equality, and both UpdateInstance predicates are pinned. `precedence-swap
ProposeSchemaChanges/proposed/reviewed` was killed, so a scenario pins `dry_run ∧ review →
proposed` (this bears on finding 8).

### Technique 4: determinism

This was the cheap form: the conformance target twice over the same 56 suites (baseline plus 55
mutants), comparing the passed and failed sets and the counts of each report. **56 of 56 were
identical.** A byte comparison of the failure messages found 2 of 56 different. In both, the only
difference is `last_run_at` (for example `2026-10-30T14:26:57Z`), which the wall clock stamps in
`src/ports.rs:184`/`:330`. The spec declares that field `{generated: true}`, so it is not a finding.
That divergence is also the technique's planted class (a clock read) showing up in the byte
comparison.

No implementation-side plant: it would need a mutated copy of `src/` and a second full build, and
the adversary charter forbids mutating the tree under attack. **So the green verdict comparison is
unproven: it was never seen to fail on a planted defect.**

Source scan of the port code a command reaches (`src/ports.rs`, `src/ask/records.rs`, `src/run.rs`,
`src/redact.rs`):
- Clock and id reads (`now_ms`, `Uuid::now_v7`) feed only `generated` fields.
- `HashMap` and `HashSet` in `run.rs` and `redact.rs` are used for keyed lookups, not iterated into
  an outcome.
- The records' list queries order by `opened_ms, rowid` or `at_ms, rowid`.

No finding.

## 4. Findings

Cover: `53170a27`; `spec/` byte-identical to `dba0b9ed`. Origin is `pre-existing`, measured on
that spec. The `src/` lines cited are unchanged since `dba0b9ed`.

| # | file:line | category | severity | verdict | origin | finding | what reaches it |
|---|---|---|---|---|---|---|---|
| 1 | spec/domains/ask.yaml:741 | spec-gap | warning | CONFIRMED | pre-existing | No view or event exposes an `AnswerRecord`'s fields. `AnswerRecorded` carries only `answer_id` and `outcome`. So 8 `sets-retarget` mutants on `RecordAnswer/recorded` survive (`answer`↔`reason`, every counter → `latency_ms`, `latency_ms` → `input_tokens`) and 2 are unwitnessed (`identity`↔`question`): the suite would pass an implementation that stores the counters crosswise | the ask service writes every answer through `RecordAnswer` (`src/ask/records.rs:979`); the review queue and the eval `replay` runner read the records back |
| 2 | spec/domains/ask.yaml:916 | spec-gap | warning | CONFIRMED | pre-existing | `CloseReview/closed` writes `expect_facts` and `expect_nodes`. A closed item leaves `OpenReviews` (`state == Open`), and nothing else observes it, so swapping the two survives. `AddEvalCase` relies on those fields (`no-expectations`, `from_review`) | `cortex ask` review close, then `cortex eval add --from-review`. `tests/eval_case_adversary.rs` checks the fields outside the conformance suite; whether it kills the swap was not measured |
| 3 | src/ports.rs:330 | missing-scenario | note | CONFIRMED | pre-existing | The `OpenReviews` `opened_at asc` → `desc` order-flip survives. `opened_at` is the wall clock stamped to the second, so every review a scenario opens ties and the `ranked` expectation cannot tell the order apart (as `tests/conformance.rs`'s own ranked test states). A clock the runner can advance would pin it. The implementation's order (`ORDER BY opened_ms, rowid`, `src/ask/records.rs:594`) was read, not measured | the review queue the ask service lists |
| 4 | spec/domains/ask.yaml:810 | spec-gap | note | CONFIRMED | pre-existing | `sets-drop CastVote/changed/value` is unwitnessed: no view over `Vote` exists, and `Voted` takes `value` from the input, so a vote change that does not persist is invisible to the suite | `POST` vote on an answer already voted on |
| 5 | spec/domains/instance.yaml:1969 | ess-limit | note | CONFIRMED | pre-existing | The two `guard-connective` mutants on `RunSource` (`fetched`, `fetch-only-failed`) are unwitnessed. One adds ESS-SYNTH-002 on `disabled`; on the other the suite does not change. The baseline `ran` scenario already sends `fetch_only: false`, so the implementation is pinned there anyway. This is worth a synthesis issue on beyond10x/ess | `cortex run --fetch-only` |
| 6 | spec/domains/ask.yaml:741 | spec-gap | note | INFEASIBLE | pre-existing | `RecordAnswer/recorded` admits negative `latency_ms`, token counts and `steps`, which break the six `>= 0` invariants at `ask.yaml:403`. The generated behaviour then returns `UnmetObligation` ("entity invariant"), an answer outside every declared outcome | nothing found: the only caller clamps `latency_ms` (`src/ask/records.rs:996`), and the gateway's usage parser maps a negative count to 0 (`src/ask/model.rs:179`) |
| 7 | spec/domains/instance.yaml:1698 | spec-gap | note | CONFIRMED | pre-existing | `seed-change-refused` is declared before `not-active`, so `UpdateInstance` on a Removed instance with a changed seed answers `seed-change-refused`, not `not-active`. No scenario pins either answer (the `not-active` scenario keeps the seed), and no precedence-swap mutant is derived for `when_subject` pairs | `cortex update <name> --spec <changed seed>` after `cortex remove <name>` |
| 8 | spec/domains/instance.yaml:2727 | spec-gap | note | INFEASIBLE | pre-existing | The command's prose says `dry_run` and `review` exclude each other, but no outcome refuses both. With both true the guards answer `proposed`, and the suite pins that (`precedence-swap` killed) | nothing found: clap refuses the pair (`src/main.rs:191`, `conflicts_with = "review"`) |
| 9 | spec/domains/instance.yaml | ess-limit | note | CONFIRMED | pre-existing | Every one of the 77 `error-swap` mutants is stillborn (ESS-COMMAND-001/004: the outcome's payload still names the old error), so this spec audits no refusal by swapping its error name. Worth an issue on beyond10x/ess | every refusal outcome of both domains |

Proposed fixes, none applied:
- 1, 2, 4: add read-only views (`Answers`, `Reviews` with closed rows, `Votes`) or carry the fields
  in events. That is spec surface plus a runner arm per view.
- 3: inject a clock into the ask ports.
- 7: decide the precedence (`not-active` first reads truer), move it, acknowledge it in
  `spec/compat-ack.*`, and pin it with an authored scenario.
- 8: a `when: {all: [dry_run, review]}` refusal, or delete the prose.

## 5. Attacked and not broken

- Guard boundaries of RecordFailure (`>= 1` both ways), every `fetch_only` equality, both
  UpdateInstance predicates and both ProposeSchemaChanges `when`s: killed when mutated.
- Every compatible emit-swap (6 of 6) and all 3 transition-moving guard mutants on RecordFailure:
  killed.
- `sets-drop` and `sets-retarget` on `UpdateInstance`, `CreateInstance`, `AdoptInstance` and
  `AddSource` (16 of 16): killed, because the `Instances` and `Sources` views publish those fields.
- Invariants of `Source` (`runs`, `consecutive_failures`): no outcome can break them.
- Determinism of verdicts: 56 suites run twice gave identical pass and fail sets.

## 6. Paths written outside the worktree

All are under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/harden/`:

| path | holds |
|---|---|
| `ir.json` | compiled IR of `spec/` |
| `gopkg/` | Go conformance package, read only for the report/2 shape |
| `plant-dead/`, `plant-shadow/` | the planted spec copies, their IR and validate output |
| `guards-baseline.txt`, `guards-plant-dead.txt`, `guards-plant-shadow.txt` | analysis output |
| `probe/run-mutants.sh` | the runner loop |
| `mut/` | the emission, every `report.json`, `manifest.json` |
| `mut2/` | the determinism rerun |
| `mutlogs/`, `mutlogs2/` | per-suite test logs |
| `mutrun.txt`, `mutrun2.txt`, `mutant-dirs.txt`, `emit.txt`, `collect.txt` | run records |
| `mutation-report.json`, `mutation-report-run1.json` | `ess-mutation-report/7` |
| `build.err`, `testbin.txt`, `fmt.txt`, `clippy.txt`, `conformance-default.txt`, `go.err` | build and check output |
| `commit-msg.zutmNn` | the commit message |
| `hardening-report-pass-1.md` | this report |

The probe `probe/guards.py` was deleted after use. Inside the worktree, `target/` grew to 937 MB.

## 7. Findings block

```findings
- file: spec/domains/ask.yaml
  line: 741
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: no view or event exposes AnswerRecord fields, so 8 sets-retarget mutants on RecordAnswer/recorded survive and 2 are unwitnessed; a crosswise store of the counters passes the suite
- file: spec/domains/ask.yaml
  line: 916
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: CloseReview's expect_facts and expect_nodes are observable nowhere once the item leaves OpenReviews, so swapping them survives although AddEvalCase relies on them
- file: src/ports.rs
  line: 330
  category: missing-scenario
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the OpenReviews opened_at order-flip survives because opened_at is the wall clock to the second and every review a scenario opens ties; the runner has no clock to advance
- file: spec/domains/ask.yaml
  line: 810
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: sets-drop of CastVote/changed value is unwitnessed because no view shows a Vote and Voted takes value from the input
- file: spec/domains/instance.yaml
  line: 1969
  category: ess-limit
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: both guard-connective mutants on RunSource are unwitnessed (ESS-SYNTH-002 added, or the suite unchanged) although the baseline ran scenario already pins fetch_only false
- file: spec/domains/ask.yaml
  line: 741
  category: spec-gap
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: RecordAnswer/recorded admits negative counters that break the six >= 0 AnswerRecord invariants and end in UnmetObligation, though no caller sends one
- file: spec/domains/instance.yaml
  line: 1698
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: UpdateInstance on a Removed instance with a changed seed answers seed-change-refused rather than not-active, and no scenario pins which
- file: spec/domains/instance.yaml
  line: 2727
  category: spec-gap
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the prose says dry_run and review exclude each other but the guards answer proposed for both, a combination only clap refuses
- file: spec/domains/instance.yaml
  category: ess-limit
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: all 77 error-swap mutants are stillborn (ESS-COMMAND-001/004), so the audit checks no refusal's error name on this spec
```

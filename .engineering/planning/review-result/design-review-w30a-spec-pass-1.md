---
format: aep.planning-md/3
id: review-result:design-review-w30a-spec-pass-1
kind: review-result
status: active
title: 'Design review (ESS hardening technique 8): spec/ against README, AGENTS.md, website/docs (wave 20261030a)'
relations:
- reviews: specification:wave-20261030a-review-fixes
revision: 1
---
# Design review: beyond10x/cortex, spec against design (ESS hardening technique 8)

- Tree read: `~/.local/state/worktree/trees/b10x/cortex/cortex-plan-reconcile-20261030`, branch `wave/20261030a`, read-only. No build, no edit. Origin compared with `git diff origin/main`.
- Spec: `spec/system.yaml`, `spec/components.yaml`, `spec/domains/instance.yaml` (3089 lines), `spec/domains/ask.yaml` (961 lines). `ess specify validate --path spec`: `cortex v1 — 4 file(s), valid` (warning: pin 0.60.0 < ess 0.77.0).
- Design: `README.md`, `AGENTS.md`, `website/docs/*.md(x)` except the generated `website/docs/reference/ess/`, `docs/handoff/2026-10-06-brain-and-cortex.md`.
- Mapping: none, so no `stale mapping` finding is possible. `UNMAPPED:` markers count as coverage and are listed under "Covered by marker".
- Paths below are relative to the tree. `S` = `spec/domains/instance.yaml`, `A` = `spec/domains/ask.yaml`, `D` = `website/docs`.

## Findings

| # | classification | design (file:line, quoted) | spec (file:line, quoted) | note |
|---|---|---|---|---|
| 1 | contradicts | `README.md:82-85` "Run a source now instead of waiting for its timer … `cortex run example/news`"; `D/commands.md:275` "`cortex source record-failure <instance>/<source> --reason <text>` \| count one failed run" | `S:1120-1144` Operator `may` lists no `RunSource` or `RecordFailure`; only "`cortex.instance.Scheduler` may: RunSource, RecordFailure" | The operator runs both commands by hand per README and quickstart; the actor model forbids it. Add both to Operator, or state the operator acts as Scheduler. |
| 2 | contradicts | `D/commands.md:115` "`ask` \| a running `cortex ask serve`. This release has none, so the runner answers `runner-failed`" | `S:794-795` "`ask` asks a running `cortex ask serve`"; `S:2382` same | Code sides with design: `src/eval.rs:429-433` always `Failure::Runner("… this cortex has no ask service to ask")`. Spec should say the runner always fails. |
| 3 | contradicts | `D/ask-service.md:178-179` "is kept as a record (`cortex.ask.AnswerRecord`) before its last event is sent" | `A:684` "The service records every question it began to answer, once its stream ended" | Code sides with design: `src/ask/serve.rs:310` "The record is kept before the last event, so a reader who saw it can vote on it". |
| 4 | contradicts | `D/operating.md:689-692` "with a `model.gateway`, exactly `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN` … and, when given, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` are set …; without one, those three are removed too" | `S:513-518` sets four (adds `CLAUDE_CODE_MAX_OUTPUT_TOKENS`); "Without a gateway, `ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN` are removed" | Code (`src/extract.rs:58-67`, `GATEWAY_VARIABLES`, four, each unset one removed) and `D/spec-file.md:167-175` agree with each other. operating.md is stale (three, not four); the spec comment understates the removal (two, not four). |
| 5 | contradicts | `D/commands.md:276` "`--kind <Web\|Connectors\|Files>`" | `S:26-28` "`variants: [Web, Connectors, Files, Structured]`" | Code sides with spec: `src/main.rs:311` `value_parser = ["Web", "Connectors", "Files", "Structured"]`. Design stale. |
| 6 | contradicts | `D/spec-file.md:905-907` "a spec that names `store.value.path` is refused by `cortex create` (`seed-refused`) and `cortex update` (`seed-change-refused`)" | `S:555-558` `SqliteStore` "`{name: path, type: Optional<String>}`", no invariant, no marker | Code refuses it (`src/main.rs:849` "store.value.path is not supported yet"). Spec admits a value the tool refuses, without an UNMAPPED marker. |
| 7 | contradicts | `AGENTS.md:51` "`spec/` is an ESS specification (`ess/20`, pinned to ESS 0.60.0" | `spec/system.yaml:1` "`format: ess/22`" | AGENTS.md stale. |
| 8 | contradicts | `AGENTS.md:64-66` "Every `UNMAPPED:` marker in `spec/domains/instance.yaml` is a rule the specification cannot state" | `A:32,74,82,114,270,296,324,342,894` nine `UNMAPPED:` markers in ask.yaml | AGENTS.md names one file; the rule applies to both. |
| 9 | contradicts | `D/ask-service.md:224-225` "An instance on a `postgres` store keeps its records in a PostgreSQL schema of their own … and writes no `ask.sqlite`" | `S:1012-1013` relation `answers`: "kept in `ask.sqlite` in its directory, never in its store" | Spec comment predates the PostgreSQL records; ask.yaml:114 marker already covers the schema. |
| 10 | missing | `D/commands.md:279-288` `ran` detail carries "`facts_refused` … `parts_rejected` … `rejected` … `too_large` … `unconverted` … `masked` … `stopped` … `redacted` … `unrestored`" | `S:2922-2938` `SourceRan` fields end at `batch_chars`, `splits`; `S:643-645` GateCheck measures name `facts_refused`, `parts_rejected`, `masked`, `truncated`, `unrestored` | Measures the gate reads have no field in the event. `too_large` is covered by marker `S:2100`. |
| 11 | missing | `D/commands.md:280` "`facts_reapplied` (facts marked `replaces: true` that had no current value to replace, applied again …)" | `S:2922-2938` `SourceRan` has no `facts_reapplied`; no marker | New in this wave (design line differs from origin/main). |
| 12 | missing | `D/operating.md:631-632` lineage held: "`create` refuses it as `seed-refused`"; `D/spec-file.md:937` password_file: "refused by `cortex create` (`seed-refused`)"; `D/spec-file.md:650-651` value_kind: "`create` answers `seed-refused`" | `S:1543-1544` "seed-refused external: EKR refuses the minimal seed, the ekr-seed/2 file or the seed schema document" | Outcome is used for refusals EKR never sees; external text names EKR only. |
| 13 | missing | `D/spec-file.md:651` "`update` answers `seed-change-refused` and changes nothing"; `D/spec-file.md:938` "`cortex update` (`seed-change-refused`)"; `D/spec-file.md:906-907` same for `store.value.path` | `S:1601-1604` `seed-change-refused` only "when_subject: seed_digest != input.seed_digest" | Code encodes these refusals into the digest (`src/main.rs:1585-1612`, "refused: …"), a trick the spec does not state. |
| 14 | missing | `D/ask-service.md:112` "`bad-request` \| 400, or 413 \| the body is not JSON with a non-empty `question`, or is above 1 MiB (413), or the question still holds a class `refuse_if_left` names (400)" | `A:586-678` `cortex.ask.Ask` outcomes: unidentified, rate-limited, busy, limit-reached, model-failed, reader-failed, cut-off, client-closed, answered; no `bad-request` | Only mentioned as a reason in `A:240-242` and as "(401, 429, 400) … leaves no record" in `A:151-152`. |
| 15 | missing | `D/ask-service.md:193-195` "A body above 64 KiB is refused with 413 `bad-request`. A failure of the records answers 500 `store-failed`" | `A:733-800` CastVote, `A:837-885` CloseReview: no `bad-request`, `store-failed` or size outcome; only "anything else answers 400" (`A:843`) | Design lines new in this wave. |
| 16 | missing | `D/limits.md:88-90` "Every batch is checked before the first model call; a class still found there fails the run (`extraction-failed`, the reason naming the class)" | `S:1895-1896` extraction-failed external: "The model call fails …, times out, exceeds the run budget before any batch, or returns no valid document" | `refuse_if_left` is a second cause with no model call; code `src/run.rs:1037` onward. |
| 17 | missing | `D/spec-file.md:859-860` "a run that fails one is undone and fails as `apply-refused`"; `D/operating.md:557-560` "(`stage-head-moved`) … the run fails as `apply-refused`"; `D/operating.md:271-273` value_kind mismatch "fails the run as `apply-refused`" | `S:1915-1916` "apply-refused external: EKR refuses the merged extraction document" | Three further causes. The gate ending `apply-refused` appears only inside marker comments (`S:1724`, `S:2002`); stage-head-moved and the run-time value_kind mismatch appear nowhere in the outcome. |
| 18 | missing | `D/spec-file.md:78-82` `budget_usd` "above 0", `daily_ceiling_usd` "above 0", `budget_tokens` "a whole number above 0", `daily_ceiling_tokens` "above 0", `timeout_s` "above 0" | `S:487-499` `ModelSpec` has no invariant; prose `S:419`, `S:434` says "above 0" | Code holds them (`src/spec.rs:303-324`). `S:457` marker covers requiredness and finiteness only. `S:1111` (`EvalRun` "`cases >= 0`") shows an integer comparison is expressible; an Optional field takes the `S:177` `any: [..., defined(...)]` shape. |
| 19 | missing | `D/spec-file.md:926` "`schema_connection` … It needs `connection`" | `S:591-596` `PostgresStore`: `connection` and `schema_connection` both Optional, no invariant | `any: ["!defined(schema_connection)", "defined(connection)"]` is the `S:177` shape. Code: `src/instance.rs:217`. |
| 20 | missing | `D/spec-file.md:36` "1 to 63 lowercase letters, digits or `-`, not starting with `-`"; `D/spec-file.md:240` source name "unique within the instance"; `D/spec-file.md:243-245` "`entities` and `seed` are reserved"; `D/operating.md:55-57` unit collision and "a source named `view` is refused" | `S:14-16` `InstanceName` "kind: newtype, of: String"; `S:397-403` `SourceSpec.name: String`; no marker | String-pattern rule with no construct and no UNMAPPED marker. The unit/`view` part is new in this wave. |
| 21 | missing | `D/commands.md:45` "New sources are added and every source's timer is (re)installed" | `S:1630-1634` updated summary: "The frozen spec is replaced; sources, model and serve settings take effect on the next run" | Create says "added by AddSource, one per source" (`S:1588`); update does not. |
| 22 | missing | `D/commands.md:59` "`cortex update` refuses a spec whose `name` is not `<name>`" | `S:1590-1640` UpdateInstance outcomes: seed-change-refused, not-active, updated, no-such-instance | Code: `src/main.rs:1582-1583` fails exit 2. |
| 23 | missing | `D/commands.md:78` "A fact the model gave no verdict for is `unclear`; a verdict for a fact the batch did not hold is dropped" | `S:2305-2308` MeasureQuality comment: "the model answers a `cortex.instance.QualityVerdict` per fact" | Gap-filling and dropping rules absent. |
| 24 | missing | `D/commands.md:187` widen_edge_type "is `invalid` … when it adds no type new to either end" | `S:950-956` `invalid` list: type/property not there, already there, inherited, placeholder | One cause missing. |
| 25 | missing | `D/commands.md:104-106` "no other key is admitted … no expected fact or node is empty or white space only" | `S:1332-1337` EvalSetInvalid: "a case of another `format`, a case without an id or a question, two cases with one id, or no case" | Extra-key and blank-expectation refusals absent. |
| 26 | missing | `D/commands.md:116` replay: "a case whose id the run recorded under another question, or a run with no `summary.json` (a failed run), is `runner-failed`" | `S:1341-1345` EvalRunnerFailed: "the run `replay` names holds no recorded answer for a case" | Two causes absent. |
| 27 | missing | `D/commands.md:44` "`adopted` reports … `seen_documents` and `seen_without_evidence`" | `S:2878-2882` `InstanceAdopted` fields `name`, `revision`, `view_port`; no `seen_*` | Reported figures not in the event. |
| 28 | missing | `D/commands.md:47` "reports the name as `before_restore`, and `state_restored` (with the reason in `state_failed` …)" | `S:2974-2977` `SnapshotRestored` fields: `name`, `snapshot` only; prose at `S:2289-2290` | Event lacks the reported fields. |
| 29 | missing | `D/spec-file.md:376-378` "A child call that fails for the same record on 3 runs in a row … the record is left out and named in `skipped`" | `S:1743` marker covers poison documents only; ChildCall (`S:143`) and RunSource state nothing on child failures | Counter rule with no marker. |
| 30 | missing | `D/commands.md:32-35` "that backend calls no model: every run of such an instance fails as `extraction-failed`, with a reason naming `story:codex-model-backend`" | `S:412-414` `ModelBackend` "variants: [Claude, Codex]", no comment | Spec presents Codex as a working backend. |
| 31 | missing | `D/spec-file.md:899` "without it, `cortex create` takes the first free port from 18900" | `S:680` `ServeSpec` "`{name: view_port, type: Optional<Integer>}`"; marker `S:673` covers range only | Default absent. |
| 32 | missing | `D/spec-file.md:314` "Without it a crawl is limited to 20 pages" | `S:402` `SourceSpec.policy: cortex.instance.FetchPolicy` (and its type) states no default | Default absent. |
| 33 | unclear | `D/operating.md:275-281` "Each stop is checked before a batch, keeps the batches before it applied, and names the limit in `stopped`"; `D/spec-file.md:130` each call gets `--max-budget-usd` | `S:1896` extraction-failed "exceeds the run budget before any batch" | Code: `src/run.rs:1088-1107` limit stops set `stopped` and end `ran`; `src/extract.rs:792` passes `--max-budget-usd`. The spec phrase may mean a first call that `claude` cuts at its budget; the design never says what that call ends as. Unclear, ask the author. |
| 34 | spec-only | `D/commands.md:238-241` "Every proposal named must be `pending` … `not-pending` decides nothing" (no blank-reason rule) | `S:2749` not-pending "… the run has no pending proposal, or the reason is blank" | Code sides with spec: `src/schema.rs:1285-1286` "a rejection needs a reason". Design should state it. |

Further design statements with no spec declaration and no marker, below table weight (each a note): quality holds the lock only while sampling (`D/commands.md:48`); files-record changes skip the refresh window (`D/spec-file.md:521`); `snapshots.keep: 0` leaves nothing to undo with (`D/operating.md:381-382`); cross-structured-source rules (`D/spec-file.md:687-688`, `706`, `754`); spec paths may not leave the spec directory (`D/spec-file.md:50`); `config` absolute or `~` (`D/spec-file.md:924`); `identity_header` must be a header name (`D/ask-service.md:56-57`). Exit statuses 0/1/2 (`D/commands.md:9-12`) are covered only for RunSource (`S:1714`).

## Counts

| classification | count |
|---|---|
| missing | 23 |
| contradicts | 9 |
| unclear | 1 |
| spec-only | 1 |
| stale mapping | 0 (no mapping exists) |
| total | 34 |

Severity: 0 blocker, 11 warning, 23 note. Origin: 2 introduced (11, 15), 1 undecided (33), 31 pre-existing. Finding 20's unit-collision and `view` half is new in this wave, but its name-rule core predates it, so it counts as pre-existing.

## Covered by marker

| design | marker |
|---|---|
| `refresh_after_days` 0 to 36500, maxima above 0 (`D/spec-file.md:817`) | `S:89` |
| not both `connection` and `profile` (`D/spec-file.md:257`) | `S:113`, `S:174`, `S:350`, `S:581` |
| window placeholders `{since}` `{until}` `{since_epoch}` (`D/spec-file.md:326`, `337-353`) | `S:161`, `S:343` |
| `value_kind` conversion (`D/spec-file.md:630-653`) | `S:217` |
| alias template grammar (`D/spec-file.md:602-606`) | `S:247` |
| `store_lookup: AnyType` (`D/spec-file.md:610-628`) | `S:255` |
| Supersede idle run (`D/spec-file.md:658-662`) | `S:371` |
| gateway requiredness, finite dollar limits (`D/spec-file.md:186-197`) | `S:457` |
| `batch_chars` at least 1000 (`D/spec-file.md:85`) | `S:484` |
| gateway refusals (`D/spec-file.md:162-165`) | `S:510` |
| `RareName` (`D/limits.md:58-75`) | `S:631` |
| `view_port` 0 to 65535 (`D/spec-file.md:899`) | `S:673` |
| hook `timeout_s` 1 to 86400 (`D/spec-file.md:893`) | `S:706` |
| review relation (`D/commands.md:149-158`) | `S:806` |
| proposal lifecycle on disk (`D/commands.md:241`) | `S:972` |
| exit statuses of a run (`D/operating.md:132-146`) | `S:1714` |
| seed run (`D/commands.md:272`, `D/operating.md:611`) | `S:1721` |
| seen documents and text hash (`D/operating.md:180-183`, `655-678`) | `S:1728` |
| poison document skipped after 3 (`D/operating.md:283-302`) | `S:1743` |
| window and `{since}` hold (`D/spec-file.md:359-384`) | `S:1762` |
| monitoring and hook (`D/operating.md:418-453`) | `S:1772` |
| known entities (`D/operating.md:229-261`) | `S:1783` |
| fetch-only run (`D/operating.md:304-328`) | `S:1813` |
| store-outcome-unknown leftovers (`D/operating.md:337-351`, `556-583`) | `S:1932` |
| revalidate retry (`D/operating.md:353-366`) | `S:1966` |
| batch-too-large (`D/operating.md:467-477`) | `S:1998` |
| SIGTERM (`D/operating.md:156-172`) | `S:2028` |
| model-unavailable classification (`D/operating.md:479-492`) | `S:2064` |
| `too_large` (`D/spec-file.md:114-117`) | `S:2100` |
| eval case add CLI and file (`D/commands.md:147-168`) | `S:2462`, `S:2465` |
| schema apply gate measures (`D/commands.md:260-262`) | `S:2795` |
| ask limits at or below 0 (`D/ask-service.md:56-57`) | `A:32`, `A:74` |
| records required and refused (`D/ask-service.md:54`, `263-266`) | `A:82`, `A:114` |
| ask figures (`D/ask-service.md:276-299`) | `A:270`, `A:296`, `A:324`, `A:342` |
| purge (`D/ask-service.md:217-220`) | `A:894` |

## Sections not reached

- `website/docs/reference/ess/*`: excluded on purpose, generated from the spec.
- `docs/handoff/2026-10-06-brain-and-cortex.md`: read; it holds process and hand-over statements, none a behavioural claim the spec could state.
- `website/docs/status.mdx` and the quickstart were read for claims only; findings from them are cited where they repeat README (finding 1).
- Everything else in scope was read in full. Not checked: the spec's prose against code beyond the lines cited.

## Planted defect

- Copy: `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/hardening/planted/spec`.
- Deleted: `S:177` in the copy, the `ConnectorsSource` invariant `- any: ["paging.stop_older_than_since != true", "defined(time)"]`.
- Design states it plainly: `D/spec-file.md:407-408` "Only a `connectors` source's own `paging` takes the rule, and only with a `time`; a spec that sets it elsewhere, or without `time`, is refused when it is loaded".
- `ess specify validate --path <copy>`: still `cortex v1 — 4 file(s), valid`. Validation does not see it.
- Procedure: for "only with a `time`", search the copy for a declaration tying `stop_older_than_since` to `time`. `grep defined(time)` returns nothing. The invariants left at `S:151`, `318`, `333`, `353` cover "elsewhere" only. Classified `missing`. Caught.
- Trap: the comment at `S:128-133` still says "(the invariants of `ConnectorsSource`, …)". A review that accepts a comment as a declaration misses the deletion. This review checks declarations.

## Findings (machine-readable)

Convention: where the spec is right and the design is stale, `file` is the design file, `category: false-positive` (no spec defect) and `verdict: NEEDS-CHANGE` (the design needs the change).

```findings
- {file: spec/domains/instance.yaml, line: 1121, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "The Operator actor may not RunSource or RecordFailure, yet README and commands.md have the operator run `cortex run` and `cortex source record-failure` by hand."}
- {file: spec/domains/instance.yaml, line: 794, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "EvalRunner `ask` and RunEval (line 2382) say it asks a running `cortex ask serve`, but src/eval.rs:429-433 always fails it as runner-failed, as commands.md:115 says."}
- {file: spec/domains/ask.yaml, line: 684, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "RecordAnswer says the record is kept once the stream ended, but src/ask/serve.rs:310 and ask-service.md:178-179 keep it before the last event so a reader can vote on it."}
- {file: spec/domains/instance.yaml, line: 516, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "Without a gateway the spec removes two variables but src/extract.rs:58-67 removes all four, and operating.md:689-692 is also stale (three variables, not four)."}
- {file: website/docs/commands.md, line: 276, category: false-positive, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "`--kind` lists Web, Connectors and Files, but SourceKind and src/main.rs:311 also admit Structured."}
- {file: spec/domains/instance.yaml, line: 558, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "SqliteStore.path is declared as accepted while src/main.rs:849 and spec-file.md:905-907 refuse it; it needs a marker or removal."}
- {file: AGENTS.md, line: 51, category: false-positive, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "AGENTS.md says ess/20 but spec/system.yaml:1 is format ess/22."}
- {file: AGENTS.md, line: 64, category: false-positive, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "AGENTS.md places UNMAPPED markers in instance.yaml only, but ask.yaml carries nine."}
- {file: spec/domains/instance.yaml, line: 1012, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "The answers relation says records live in ask.sqlite, but a postgres instance keeps them in a PostgreSQL schema and writes no ask.sqlite (ask-service.md:224-225)."}
- {file: spec/domains/instance.yaml, line: 2938, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "SourceRan lacks facts_refused, parts_rejected, masked, truncated, unrestored, stopped, rejected, unconverted and redacted, which the run reports and the gate (line 643) measures."}
- {file: spec/domains/instance.yaml, line: 2938, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: introduced, message: "commands.md:280 adds a facts_reapplied report field that SourceRan does not carry."}
- {file: spec/domains/instance.yaml, line: 1544, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "seed-refused is also the answer for a held lineage, a password_file config, store.value.path and a value_kind mismatch, none of which its EKR-only external text names."}
- {file: spec/domains/instance.yaml, line: 1603, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "seed-change-refused fires only on a seed digest change, but update also answers it for value_kind, password_file and store.value.path refusals, which src/main.rs:1585-1612 encodes into the digest."}
- {file: spec/domains/ask.yaml, line: 586, category: missing-scenario, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "Ask has no bad-request outcome for a non-JSON or empty question, a body above 1 MiB, or a refuse_if_left class, all documented in ask-service.md:112."}
- {file: spec/domains/ask.yaml, line: 733, category: missing-scenario, severity: note, verdict: NEEDS-CHANGE, origin: introduced, message: "The record routes' 413 bad-request above 64 KiB and 500 store-failed (ask-service.md:193-195) have no outcome on CastVote or CloseReview."}
- {file: spec/domains/instance.yaml, line: 1896, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "extraction-failed does not name the refuse_if_left check that fails a run before any model call (limits.md:88-90)."}
- {file: spec/domains/instance.yaml, line: 1916, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "apply-refused names only an EKR refusal, not a failed gate, stage-head-moved or a value_kind mismatch at run time, all of which end as apply-refused."}
- {file: spec/domains/instance.yaml, line: 499, category: spec-gap, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "ModelSpec states budget_usd, daily_ceiling_usd, budget_tokens, daily_ceiling_tokens and timeout_s above 0 only in prose; no invariant or marker, though src/spec.rs:303-324 enforces them."}
- {file: spec/domains/instance.yaml, line: 596, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "PostgresStore lacks the invariant that schema_connection needs connection (spec-file.md:926, src/instance.rs:217)."}
- {file: spec/domains/instance.yaml, line: 16, category: ess-limit, severity: warning, verdict: NEEDS-CHANGE, origin: pre-existing, message: "Instance and source name rules (pattern, uniqueness, reserved names, unit collisions, `view`) have no declaration and no UNMAPPED marker."}
- {file: spec/domains/instance.yaml, line: 1630, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "UpdateInstance's updated summary does not say new sources are added and every timer is reinstalled (commands.md:45)."}
- {file: spec/domains/instance.yaml, line: 1600, category: missing-scenario, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "UpdateInstance has no outcome for a spec whose name differs from the instance, which src/main.rs:1582 refuses."}
- {file: spec/domains/instance.yaml, line: 2308, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "MeasureQuality does not say a fact without a verdict is unclear and a foreign verdict is dropped (commands.md:78)."}
- {file: spec/domains/instance.yaml, line: 956, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "The invalid status omits a widen_edge_type that adds no new type to either end (commands.md:187)."}
- {file: spec/domains/instance.yaml, line: 1336, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "EvalSetInvalid omits extra keys and blank expected facts or nodes (commands.md:104-106)."}
- {file: spec/domains/instance.yaml, line: 1344, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "EvalRunnerFailed omits a replayed case recorded under another question and a replayed run with no summary.json (commands.md:116)."}
- {file: spec/domains/instance.yaml, line: 2882, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "InstanceAdopted lacks the seen_documents and seen_without_evidence that adopted reports (commands.md:44)."}
- {file: spec/domains/instance.yaml, line: 2977, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "SnapshotRestored lacks before_restore, state_restored and state_failed, which restore reports (commands.md:47)."}
- {file: spec/domains/instance.yaml, line: 1743, category: ess-limit, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "The rule that a child call failing on 3 runs skips its record (spec-file.md:376-378) has no marker; the poison-document marker covers documents only."}
- {file: spec/domains/instance.yaml, line: 412, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "ModelBackend Codex is declared with no word that every run, quality and schema call with it fails (commands.md:32-35)."}
- {file: spec/domains/instance.yaml, line: 677, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "ServeSpec.view_port does not state the default, the first free port from 18900 (spec-file.md:899)."}
- {file: spec/domains/instance.yaml, line: 402, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "No default of 20 pages for a crawl without a policy (spec-file.md:314) is declared."}
- {file: spec/domains/instance.yaml, line: 1896, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: undecided, message: "\"exceeds the run budget before any batch\" is unclear against operating.md:275-281 and src/run.rs:1088-1107, where limit stops end ran; ask the author what it covers."}
- {file: website/docs/commands.md, line: 240, category: false-positive, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "reject with a blank reason answers not-pending (spec line 2749, src/schema.rs:1285); the design does not say so."}
```

---
format: aep.planning-md/3
id: review-result:adversary-w30a-ess-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit ess (wave 20261030a)'
relations:
- reviews: story:ess-pin-0-77
- reviews: story:spec-compat-gate
revision: 1
---
unit: `ess` of cortex wave 20261030a, branch `unit/w30a-ess` at `c2a7914a` (base `effef82c`)
verdict: CONFIRMED (3 notes, no blocker, no warning)
cases: executed n/a→n/a, red 0 (no Rust case added: every finding is in a document and is shown by a command; no suite run)
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/ess/adv2/`
needs-coordinator: none

**1. `git --no-pager diff --stat`:** empty. `git status --short` shows 0 lines. HEAD is still `c2a7914a`, and I made no commit.

**2. Cases added:** none. `tests/w30a_ess_adversary.rs` does not exist. No code path in the unit gave a red case (part 5), and the brief routes doc findings to commands.

**3. Suite run:** none, because I added no case.

**4. Findings** (they cover `c2a7914a`)

1. **`spec/compat-ack.md:20` (`max_tool_calls`) and `:19` (`max_in_flight`): the reasons describe how the spec parses but leave out a behaviour change against v0.6.1.** CONFIRMED, introduced, note.
   - **What was measured:** v0.6.1 had no tool-call cap per step and no in-flight cap:
     - `git grep -c 'max_tool_calls\|max_in_flight' v0.6.1 -- src` returns 0 files.
     - The v0.6.1 loop makes every call in `reply.tool_calls` (`git show v0.6.1:src/ask/mod.rs`, lines 482-560).
     - HEAD sets `DEFAULT_MAX_TOOL_CALLS = 16` and `DEFAULT_MAX_IN_FLIGHT = 8` (`src/ask/mod.rs:199`, `:202`).
   - A v0.6.1 spec without these fields therefore behaves differently now:
     - Calls past 16 in one step are not made.
     - A 9th concurrent answer gets 429 `busy`.
   - The rows say only "parses as before and takes the default 16/8". Every other `Optional` row says "as before" about behaviour.
   - The `busy` row covers `max_in_flight`. Nothing covers the tool-call cap.
   - **What reaches it:** a reviewer reading `compat-ack.md` as the record of what 1.0 changes for an existing spec. CHANGELOG.md:197-201 does state both caps.
   - **Fix:** in the `max_tool_calls` row, add that 0.6.1 made every call.

2. **`AGENTS.md:73-76`: the reset instruction cannot be followed as written.** CONFIRMED, introduced, note.
   - **What was measured:** AGENTS.md says to set `before` and `after` to "the digests `ess verify diff … --format json` prints". That JSON prints objects, not digests:
     ```
     ess verify diff --from spec --to spec --format json | jq .before
       -> {"system":"cortex","specification_version":1,"spec_digest":"454ae2cd…"}
     ack file with before/after copied as those objects:
       refused: …: acknowledgements: invalid type: map, expected a string at line 3 column 12   EXIT=1
     ack file with .before.spec_digest / .after.spec_digest and acknowledged []:
       gate --fail-on breaking-or-unknown: passed (0 acknowledged)                             EXIT=0
     ```
   - **What reaches it:** an agent doing step 7 of the release procedure after the next tag.
   - **Fix:** say `.before.spec_digest` and `.after.spec_digest`.

3. **`website/data/status.json:33`: the public status page still says "115 conformance scenarios".** CONFIRMED, pre-existing, note.
   - **What was measured:** the suite now has 124 (`jq '.scenarios|length' spec/suite.json`).
   - At base it had 120, and `git show effef82c:website/data/status.json` already said 115.
   - The unit updated the count in AGENTS.md and CHANGELOG.md but not here. The file is hand-written and nothing generates or checks it.

**5. Attacked and could not break**
- **The correction's AGENTS.md exit statuses:** all hold, measured with `ess` directly.
  - An id the delta does not hold: exit 1 (`[undeclared_reference]`).
  - An id listed twice: exit 1 (`[duplicate_declaration]`).
  - One acknowledgement removed: exit 4, with `fails --fail-on breaking-or-unknown: event/…facts_reapplied`.
  - No acknowledgements file: exit 4.
  - Pass 1 measured `task`'s 201.
- **The 6 corrected rows and the CHANGELOG Changed bullet, against v0.6.1:**
  - v0.6.1 `store_failed` put the store's text into `message` (`git show v0.6.1:src/ask/serve.rs` :78-80).
  - HEAD sends a fixed message plus `request_id`, and the detail goes through `log` to standard error (`src/ask/serve.rs:92-105`).
  - `facts_reapplied` is in no v0.6.1 file.
  - The close route's 400 cases match v0.6.1 :341-373.
  - One nuance, read from code and not run: v0.6.1 already answered 413 above 1 MiB through the router's `DefaultBodyLimit` (:135, :462). That body was axum's plain text, not JSON. The rows' "no limit of its own" is still accurate.
- **The other 16 rows, checked against v0.6.1 source:** each holds.
  - `client-closed`: v0.6.1 had `gone()` as `model_failed`, which opened a `Failed` review.
  - `cut-off`: did not exist in v0.6.1.
  - `store-outcome-unknown`: v0.6.1 recorded it as `Refused`, and the CHANGELOG says so.
  - `rekey-refused`: `keyed_by` compares fields that v0.6.1 already had.
  - `busy`: v0.6.1 already had a 429 `rate-limited`.
  - The `Optional` rows: the fields are absent at v0.6.1, and the defaults are as stated.
- **The full delta against v0.6.1 (`--format json`):** every breaking or unknown change is in the 22 acknowledgements. Everything else is classified compatible.
- **Generated decoders under 0.77.0 and D19:**
  - cortex reads a `cortex.instance/1` spec only through serde into `cortex-spec-types` (`src/spec.rs:170-188`).
  - That crate's `types.rs` diff is only enum identifiers (`V0` → `Claude`, …) plus the header. Every `#[serde(rename)]` string is unchanged, and `Cargo.lock` is unchanged.
  - The new `json::admitted` and `json::parse_written` in `cortex-model` have no caller, and nothing in production decodes caller input through the generated `json` module.
  - `examples/` is byte-identical to v0.6.1 (`git diff --stat v0.6.1 HEAD -- examples` is empty), and `tests/spec_compat.rs` and `tests/agent_tooling.rs` load those files.
  - So a stored v0.6.1 spec keeps loading. This is shown by code and diff; I did not run it, since no build was needed to settle it.
- **Ask request bodies:** decoded by hand from `serde_json::Value` in `serve.rs`, not by generated code. The new `BadRequest`/`StoreFailed` arms in `records.rs` and `serve.rs` cannot be reached in production, because `external()` defaults to false (`src/ports.rs:335-342`).
- **Other workflows:**
  - `release.yml`, `pages.yml` and `b10x-docs-site.yml` neither install ESS nor run `task check`.
  - `release.yml`'s publish job already uses `fetch-depth: 0`.
  - `git grep 0.60.0` outside history, CHANGELOG and `.engineering` finds only the dated STATUS.md row from 2026-10-21, which is true as of its date.
- **`task check` order:** the order is validate, then `drift`, then `compat`. `compat` reads only `spec/` and `git archive <tag> spec`, never `generated/`, so a stale generated tree cannot affect it, and `drift` fails first anyway.
  - Measured with `ess` 0.77.0: `ess specify validate --strict-requires` exit 0, `task drift` exit 0, `task compat` exit 0 (22 acknowledged).
- **`tests/compat_ack.rs`:** it can fail. A missing row, an extra row, a duplicate or an empty reason each trips an assertion, so it is not one of the five shapes that cannot fail.

**6. Paths written outside the worktree:** `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/ess/adv2/`, which holds:
- `base/spec` (the v0.6.1 spec, extracted)
- 6 `ack-*.json` probe files
- `diff.json`, `self.json`, `diff.err`
- 10 `*.log` files
- `tmp/` (empty)

No process was left running. I released the session lease `adv-w30a-ess-pass2`.

```findings
- file: spec/compat-ack.md
  line: 20
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the max_tool_calls row says only that a spec without the field parses as before, but v0.6.1 made every tool call of a step and 1.0 caps a spec without the field at 16, which no acknowledgement reason states
- file: AGENTS.md
  line: 73
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the reset step says to copy the digests ess verify diff --format json prints, but its before/after are objects and copying them is refused with exit 1; the strings are .before.spec_digest and .after.spec_digest
- file: website/data/status.json
  line: 33
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the public status page says 115 conformance scenarios while spec/suite.json has 124 (120 at base), and the unit updated the count only in AGENTS.md and CHANGELOG.md
```

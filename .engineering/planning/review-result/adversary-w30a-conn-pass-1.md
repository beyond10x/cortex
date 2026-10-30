---
format: aep.planning-md/3
id: review-result:adversary-w30a-conn-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit conn (wave 20261030a)'
relations:
- reviews: story:connectors-invoke-0-60
revision: 1
---
verdict: fail

None of the findings blocks the merge. I added one failing case and committed it as ignored, so the branch stays green.

```
unit: conn (story:connectors-invoke-0-60), findings cover 81205fa5; adversary commit 1d7741cc on top
verdict: CONFIRMED (warning, follow-up)
cases: executed 46→46 in the targeted suites (the new case is ignored), red 1 (run with --ignored)
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (listed in part 6)
needs-coordinator: whether finding 1 holds the unit; the story's Decision says connector/instance appear "since 0.60.0", but they are listed from Connectors 0.54.0 on
```

**1. Diff stat** (`git --no-pager diff --stat 81205fa5..HEAD`)
```
 tests/connectors_invoke_0_60_adv.rs | 70 +++++++++++++++++++++++++++++++++++++
 1 file changed, 70 insertions(+)
```
Only a test file. Commit `1d7741cc test: adversary case for a Connectors older than 0.60.0`, author and committer `b10x-bot[bot]`; the file and message passed scan-text.

**2. The case added**

`tests/connectors_invoke_0_60_adv.rs::adv_a_connectors_older_than_0_60_that_lists_connector_and_instance_is_named_in_the_reason`
- **Finding:** follow-up (warning).
- **Stand-in:** behaves like Connectors 0.59.3. Its list entry carries `connector` and `instance`, and `invoke` gets the reply the real 0.59.3 binary gave to cortex's exact command line: `{"error":{"code":"cli_parse","data":{}},"ok":false}`, exit 2.
- **Assertion:** the run's reason names either "0.60.0" or `cli_parse`.
- **Status:** red now, ignored on the branch.
- **Command:** `cargo test --test connectors_invoke_0_60_adv -- --ignored`
- **Result:**
  - `the reason names neither the Connectors version cortex needs nor the refusal: tavily.websearch.search: unknown at unknown`
  - `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s`

**3. Suite run** (after the case existed)
- **Command:** `cargo test --test connectors_invoke_0_60_adv --test connectors_invoke_0_60 --test connectors_admission_retry --test connectors_admission_retry_adv --test structured_children_adv --test e2e --test agent_tooling --test connection_profile_adv` gave `EXIT1=0`. Every target reported `test result: ok`: 3, 7, 2, 5, 4, 0 passed with 1 ignored, 14 and 11 passed.
- **The ignored case on its own:** `test result: FAILED. 0 passed; 1 failed` (`EXIT2=101`).

**4. Findings**

| # | file:line | finding | verdict | origin |
|---|---|---|---|---|
| 1 | `src/connectors.rs:540` (`target`), `:144` (`envelope`) | The "Connectors 0.60.0 or newer" refusal never fires against a real older Connectors, because every release from 0.54.0 on already lists `connector` and `instance`. An older Connectors therefore gets the 0.60.0 command line and refuses it with `cli_parse`, and `envelope` reports `unknown at unknown` because it reads only `error.data.code`. | CONFIRMED | introduced |
| 2 | `tests/common/mod.rs:89`, `tests/connectors_invoke_0_60.rs:4` | Both comments say every `operations …` command is a parse refusal in 0.60.0, but `operations describe` still exists there. No behaviour depends on this. | CONFIRMED | introduced |
| 3 | `src/connectors.rs:523` | cortex puts the operation before the options with no `--`. Connectors' own id rule accepts an id that starts with `-`, and such an id gets `cli_parse`. | INFEASIBLE | introduced |

**Finding 1**
- **What I measured:** the red case above. I also ran the real 0.59.3 binary at `~/.cargo/bin/connectors` with `invoke tavily/main websearch.search --connection conn_test --input-stdin --output json`; it returned `{"error":{"code":"cli_parse","data":{}},"ok":false}` and `exit=2`.
- **The version premise:** in the connectors repository, `connectors.cli.ConnectionSummary` in `ess/domains/cli.yaml` has `connector` and `instance` at every tag from v0.54.0 through v0.59.3. Every tag up to v0.53.1 lacks them.
- **What reaches it:** any operator who upgrades cortex while still on Connectors 0.54–0.59.x. This machine has 0.59.3 installed.
- **Suggested fix (not applied):**
  - When `data.code` is absent, `envelope` should fall back to `error.code`.
  - A `cli_parse` on `invoke` should be reported as "needs Connectors 0.60.0 or newer".
  - The doc comment at `:538`, the CHANGELOG and the story Decision should say the fields come from 0.54.0 on, not 0.60.0.

**Finding 3**
- **What I measured** (real 0.60.0 binary):
  - `invoke tavily/main -op --connection conn_test --input-stdin --output json` returned `cli_parse`, exit 2.
  - `invoke --connection conn_test --input-stdin --output json -- tavily/main -op` reached admission (`not_found`, exit 1).
- **What reaches it:** nothing I found; no known operation id starts with `-`.

**The changed assertion in `structured_children_adv.rs`:** the change is sound. In 0.60.0 a read result is the JSON value itself, never JSON text inside a string (`read_answer` in `crates/connectors-host/src/local/owner/supervisor.rs`). So the old case of a result string that is not JSON no longer happens. The test still covers what it was for: a child failure other than `forbidden` is skipped and does not fail the run.

**5. What I attacked and could not break**
- **Command line against the real parser:**
  - **The invoke argv:** cortex's exact `invoke` argv (with `--connection`, `--input-stdin` and `--output json` last) parses in the real 0.60.0 binary and reaches admission. An unknown extra flag is `cli_parse`, exit 2.
  - **The removed form:** `operations invoke` is `cli_parse`, exit 2.
  - **The other commands cortex sends:** `connections list`, `revalidate` and `status` with `--adapter` all parse and add a `deprecations` member. cortex ignores that member.
- **Result envelope:** 0.60.0 sends `{connector, instance, operation, revision, result: <JSON value>}` (`invoke.rs:811-817`, `supervisor.rs:2103-2107`). The stand-ins now match that.
- **Error envelopes and retries:** `not_granted`, `not_found` and `timeout` are still reported at stage `admission` (`owner_failure` in `apps/connectors/src/local.rs`), so cortex's renew and retry still apply. A refusal still goes to stderr with stdout empty.
- **Standard input:**
  - **When it is written:** the input is written by a separate thread and then the pipe is closed. A child that never reads it causes a broken pipe, which is ignored.
  - **Connectors' own child processes:** the owner process that 0.60.0 starts gets an empty stdin (`transport.rs:1440`), so it cannot hold the pipe open.
  - **Where the input travels:** the same bytes are sent again on a retry, and no input field appears in argv.
- **A list entry with only one of connector/instance:** refused by `target()`, and the unit test covers each half and an empty value.
- **The 19 migrated stand-ins:**
  - **Narrowing:** none got looser. The old `*"--operation X"*` arms became `"invoke "*" X "*`, the shared stand-in matches `invoke tavily/main` exactly, and every stand-in still exits 2 on an unknown command, so a description read put back before a call would fail the tests.
  - **Where tests read the input:** the ones that read it now read argv plus stdin, and the dedicated test in `connectors_invoke_0_60.rs` checks that nothing is in argv.
- **Records path `$`:** an operation answering a bare array is now refused, but no Connectors answer is shaped that way; catalog outputs are objects (`{status, body, provenance}`).

I did not take a worktree session lease; I only read the tree and added this one test file.

**6. Paths written outside the worktree**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/conn/adv/` (84 MB): the downloaded 0.60.0 release archive (checksum verified), the extracted binary, scratch Connectors configs and state for 0.60.0 and 0.59.3 (`home/`, `home59/`), `run-case.log` and `run-suite.log`. No process was left running.
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/conn/commit-adv.MYCxS4`: the commit message.

**7. Findings block**
```findings
- file: src/connectors.rs
  line: 540
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Connectors 0.54.0-0.59.x already list connector and instance, so the 0.60.0 guard never fires and their cli_parse refusal of the new invoke command line is reported as "unknown at unknown"
- file: tests/common/mod.rs
  line: 89
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the stand-in comments say every operations command is a parse refusal in 0.60.0, but operations describe still exists there
- file: src/connectors.rs
  line: 523
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: an operation id starting with "-", which Connectors' id rule admits, is put before the options without "--" and refused as cli_parse; no known operation is named that way
```

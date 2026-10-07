---
format: aep.planning-md/3
id: review-result:security-postgres-credential-from-connectors-pass-1
kind: review-result
status: active
title: 'Security review pass 1: postgres-credential-from-connectors (wave 20261006i)'
relations:
- reviews: story:postgres-credential-from-connectors
revision: 1
---
```
unit: story:postgres-credential-from-connectors
verdict: red
cases: executed 23→29, red 4
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

```
 tests/store_backend.rs | 346 +++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 346 insertions(+)
```
That is the only change in the tree, and it is a test file. Nothing in `src/` was edited. This review covers the working tree at `ece3755` plus this test diff, reviewing the range `30fd665..8514d95`.

**The password is not leaked.** Invariants 1 and 4 hold, and 3 holds wherever I tested it. The four red cases are gaps against invariants 2, 5 and 6, and none of them puts the password in anything cortex writes.

## Findings

| id | file:line | severity | verdict / origin | what | reach |
|---|---|---|---|---|---|
| F1 | src/ekr.rs:212 | note | CONFIRMED / introduced | Any `EKR_*` variable cortex inherited from its own environment goes through the launch to `ekr`. Invariant 2 says only the three cortex sets should. | The operator's shell environment. EKR 0.0.32 reads only `EKR_HOST`, `EKR_BACKEND`, `EKR_STORE` and `EKR_FULL_REPLAY` (`strings` on the binary), so the only real exposure is `EKR_FULL_REPLAY`. |
| F2 | src/instance.rs:243 | warning | CONFIRMED / introduced | `launch_refusal` only checks `password_file`. A config that also has a `password` key is accepted: `update` answers `updated`. | `cortex update` (main.rs:1249) starts no `ekr`, so nothing refuses it. `create` and `adopt` are refused later by EKR's own error ("invalid ekr.postgres/1 document", which I measured), and that error does not name the field. |
| F3 | src/main.rs:535 | note | CONFIRMED / introduced | When the spec names `schema_connection`, the `--postgres-schema-config` file is never checked for `password_file: /proc/self/fd/3`. Provisioning then launches with the schema role's password still in the operator's own file. | `cortex create --postgres-schema-config` together with `schema_connection` |
| F4 | src/main.rs:1335 | warning | CONFIRMED / introduced | Every `update` that installs units now rewrites the viewer unit and runs `enable --now` on it, including a SQLite store whose store did not change. The base version never touched the viewer (main.rs:1229/1319 at `30fd665`). This contradicts operating.md:284 ("when an update adds or drops the connection") and `restart_view`'s rule that a viewer the operator stopped stays stopped (schedule.rs:587). | Any `cortex update` without `--no-units` |

## Cases added (tests/store_backend.rs)

| test | line | asserts | now |
|---|---|---|---|
| `security_every_ekr_that_opens_a_launched_store_holds_the_connection_on_descriptor_3` | 1893 | Every `ekr` that opens the store during `create`, `run` (with gate), `quality` and `schema` has the connection document on fd 3, checked with `readlink`. The verbs `head`, `seed`, `apply-extraction`, `quality` and `sample` all appear. | green |
| `security_an_inherited_ekr_variable_does_not_reach_a_launched_ekr` | 1942 | F1 | red |
| `security_update_refuses_a_launched_config_that_also_carries_a_password` | 1977 | F2 | red |
| `security_create_refuses_a_schema_config_that_takes_no_password_from_the_launch` | 2025 | F3 | red |
| `security_an_update_of_a_store_without_a_connection_leaves_the_viewer_alone` | 2061 | F4 | red |
| `security_the_mcp_line_keeps_hostile_connection_words_one_word_each` | 2096 | Run through `sh`, the MCP line keeps a hostile adapter (`-a'd $HOME \ "q" %i`) and connection id (`-c⏎next; $(id) \`id\``) as one word each | green |

Output of each case run alone, verbatim:
```
security_every_ekr_…descriptor_3 ... ok   | test result: ok. 1 passed; 0 failed; … 28 filtered out
security_an_inherited_ekr_variable_does_not_reach_a_launched_ekr:
  assertion `left == right` failed: EKR_REVIEW_INHERITED, which cortex did not set, reached the launched ekr head:
  postgres|…/connectors-state/app.json|from-the-operator-shell|head
  left: "from-the-operator-shell"  right: "&lt;unset&gt;"
security_update_refuses_a_launched_config_that_also_carries_a_password:
  assertion `left == right` failed: an ekr.postgres/1 with store.value.connection and a password was accepted: {"command":"update","detail":{"added_sources":[],"name":"pg"},"outcome":"updated"}
  left: (Some(0), Some("updated"))  right: (Some(1), Some("seed-change-refused"))
security_create_refuses_a_schema_config_that_takes_no_password_from_the_launch:
  left: (Some(0), Some("created"))  right: (Some(1), Some("seed-refused"))
security_an_update_of_a_store_without_a_connection_leaves_the_viewer_alone:
  an update that changes no store enabled and started the viewer:
  --user daemon-reload
  --user enable --now cortex-lite-view.service
security_the_mcp_line_keeps_hostile_connection_words_one_word_each ... ok
```

## Suite run
`cargo test -p cortex-cli --test store_backend` (with `CORTEX_TEST_EKR=&lt;worktree&gt;/target/ekr-0.0.32/ekr`), Docker case included:
```
running 29 tests
test result: FAILED. 25 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.06s
CARGO_EXIT=101
```
The before count comes from the same command with `-- --skip security_`: `running 23 tests`, `test result: ok. 23 passed; … 6 filtered out`, exit 0. No container with the label `cortex.test=store-backend` was left (`docker ps -a` count 0).

## Proposed fixes (patches only, not applied to the tree)
- `target/wave-scratch/security/inherited-ekr-env.patch` (F1): `Launch::command` removes every inherited `EKR_*` variable before the caller sets its three.
- `target/wave-scratch/security/launch-config-refusals.patch` (F2, F3): a shared `password_file_refusal` that also refuses a `password` key. `seed_instance` checks the schema config when `schema_connection` is set. The refusal messages name fields, never values.
- `target/wave-scratch/security/update-viewer-only-on-relaunch.patch` (F4): `update` reinstalls the viewer only when the store's `launch` changed.

I tested all three patches applied together to a copy of the tree in scratch: `store_backend` 29/29, `--lib` 164, `e2e` 11, `adopt` 13, `unit_homes` 4, `unit_homes_adversary` 12, `seed_change` 9, all passing and fmt-clean. I did not test each patch on its own.

## Checked and found no fault
- **Invariant 1:** cortex never reads the password document, and `launch_refusal` throws away serde errors without printing them. The Docker case passed: the password is not under the home, in the units, in the argv logs or in the MCP line.
- **Invariant 3, systemd units:** `exec_quote` and `env_line` match systemd.syntax(7) as I read it. Newlines are refused by `single_line`, and instance names are restricted to `[a-z0-9-]`.
- **Invariant 3, launch argv:** the real `connectors` 0.31.0 refuses adapter values with a leading `-` or `--` at parse time (`cli_parse`, exit 2), so they cannot split the command.
- **Invariant 4:** I grepped every `Command::new` in `src/`. The only `ekr` calls that bypass the launch open no store (`example`, `schema`, `fact-quality`), and `restore` refuses postgres (snapshot.rs:573).
- **Invariant 5, the other two refusals:** a missing `password_file` and a `schema_connection` without `connection` are both refused through the same `store_refusal`, in create (main.rs:495), update (1249) and adopt (1079).
- **Invariant 6:** `Store::cmd` without a launch builds the same command as at base.
- **Not raised:** the viewer unit of a launched store now copies every `CONNECTORS_*` variable into it (source units already did this at base). I don't know whether Connectors keeps any secret in those variables, because a guard blocked me from reading its repository.

## Paths written outside the worktree
None by me. All scratch, logs and the patched copy (443M in its own `target/`, left in place) are under `target/wave-scratch/security/`. The tools wrote their own state: the `worktree hook` lease records (location not inspected), cargo's package-cache lock, and Docker containers that the tests created and removed.

```findings
[
  {"file": "src/ekr.rs", "line": 212, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "F1: An EKR_* variable cortex inherited reaches the launched ekr through pass_env, because the launch command does not remove inherited EKR_* variables."},
  {"file": "src/instance.rs", "line": 243, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "F2: launch_refusal checks only password_file, so cortex update accepts a launched ekr.postgres/1 that also carries a password key."},
  {"file": "src/main.rs", "line": 535, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "F3: With schema_connection set, create launches provisioning with a --postgres-schema-config file that is never checked for password_file /proc/self/fd/3."},
  {"file": "src/main.rs", "line": 1335, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "F4: Every update with units rewrites the viewer unit and runs enable --now on it, even for a store without a connection, contradicting operating.md:284 and restart_view's rule that a stopped viewer stays stopped."}
]
```

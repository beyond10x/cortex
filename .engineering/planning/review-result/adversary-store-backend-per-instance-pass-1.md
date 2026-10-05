---
format: aep.planning-md/3
id: review-result:adversary-store-backend-per-instance-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: store-backend-per-instance (wave 20261005c)'
relations:
- reviews: story:store-backend-per-instance
revision: 1
---
unit: U1 story:store-backend-per-instance, uncommitted working tree `cortex-w2-store` on base 6352006
verdict: NEEDS-CHANGE
cases: executed 52→56, red 4
origin: introduced 4 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 directory (`~/.cache/cortex-wave-20261005c/store/adversary-1/`), plus the assigned build dir; details in part 6
needs-coordinator: the acceptance names a GitHub Actions run URL as its evidence. No run exists for this uncommitted tree, and pushing is the coordinator's job.

**1. Diff stat**

`git --no-pager diff --stat` is unchanged from what I was handed (ekr.rs 98, instance.rs 11, main.rs 78, schedule.rs 12). `git status --short` shows one added path, the untracked `tests/store_backend_adversary.rs` (headed "ADVERSARY CASES"). I changed no implementation file.

**2. Cases** (each run alone first; logs are `adversary-1/red-<test>.log`, all EXIT=101, all still red)

| case | asserts | red line (verbatim, `$HOME` shortened) |
|---|---|---|
| `adversary_a_second_home_does_not_create_an_instance_over_another_homes_postgres_store` | a second cortex home creating `pg` on the same schema is not reported `created` over the first home's history | `left: Some("created")`; second home's head `{"revision":3,…}`, identical to the first home's head |
| `adversary_a_sqlite_store_path_the_spec_names_is_where_the_store_is` | `store: {backend: sqlite, value: {path: X}}` → the store is at X, or the create is refused | `the spec names the store ~/…/elsewhere/brain.sqlite; cortex created it at ~/…/instances/lite/store.sqlite (exists: true)` |
| `adversary_an_update_that_names_another_sqlite_file_is_refused` | an update naming another SQLite file is not `updated` | `left: Some("updated")` |
| `adversary_a_connection_string_in_the_config_field_is_not_printed_back` | a refusal never prints the password | `stdout {"…reason":"store.value.config \"postgres://ekr_app:adversary-pw-0e1f@db.example:5432/ekr\": write an absolute path…"}` |

**3. Suite** (run after the cases existed)
- `task check > adversary-1/suite.log`: `task: Failed to run task "test": exit status 101` / `EXIT=201`. Cargo stopped at the first red binary: `test result: FAILED. 0 passed; 4 failed`.
- To get the count, `cargo test --locked --workspace --no-fail-fast` (`suite-no-fail-fast.log`): EXIT=101, 56 executed. The 52 earlier cases are all green and the 4 new ones are red. The 52 comes from the implementor's `gate.log`.

**4. Findings** (they cover the working tree above)

| # | file:line | verdict / origin | measured | what reaches it |
|---|---|---|---|---|
| F1 | src/main.rs:386 | NEEDS-CHANGE / introduced | case 1 red. Probe `tenant.log`: tenant = instance name (ekr.rs:113), and re-seeding the same default seed returns the first seed's event (exit 0) | two homes (two operators or machines) using one shared schema, the same instance name and the default seed. Fix: before seeding Postgres, require that `ekr head` reports "the lineage has no seed", otherwise refuse with seed-refused |
| F2 | src/instance.rs:81 | CONFIRMED / pre-existing | case 2 red. At the base, `store_handle` always uses `store.sqlite` (read with `git show`, not run) | the schema and `STORE_FORMS` advertise `value: {path}`. Fix: honour it, or refuse it |
| F3 | src/main.rs:571 | CONFIRMED / pre-existing | case 3 red. The comparison uses resolved handles, which drop the SQLite path. The base compared only the seed | `cortex update` with a spec naming another file. Fixing F2 by refusing only at create leaves this open |
| F4 | src/main.rs:356 | CONFIRMED / introduced | case 4 red. The leak goes to stdout only; the registry stores no reasons (home.rs `to_json`) | an operator puts the connection string where the config path belongs. Fix: name the field, not its value |
| F5 | tests/store_backend.rs:132 | CONFIRMED / introduced | probe `interrupt.log`: after SIGTERM, `left after interrupt: cortex-store-backend-2360331-… Up 3 seconds`. A panic does not leak; Drop runs | Ctrl-C of a local `task check`, or a CI timeout (the runner is thrown away, so CI is unaffected). Fix: a label and a sweep of stale containers, or a lifetime limit inside the container |
| F6 | src/main.rs:450 | CONFIRMED / introduced | seed-refused deletes the directory, but the Postgres lineage stays, so the spec's "no store was kept" is false. A retry with a different seed gets `ekr.kernel.AlreadySeeded` (probe). I read the cortex path; I did not run it end to end | a rejected seed schema, after which the seed is edited |

**5. Attacked and could not break**
- **Leaks:** ekr's errors redact the password for 5 URL forms and 3 config forms (`probe-errors.log`). The ekr child's argv and environment carry only paths. Units, the MCP line and the frozen spec carry no secret.
- **Backend use:** the view unit, the MCP line and the timers all follow the spec's backend; timers go through the frozen spec at run.rs:168. `ekr view` serves a Postgres store (HTTP 200).
- **Update:** changing the backend is refused, and `~/` and the equivalent absolute path count as the same store.
- **Relative config path:** refused before the instance directory is created.
- **SQLite:** the view-unit and MCP-line formats are byte-identical to the base, and the 4 spec_compat cases pass.
- **Isolation and CI:** different instance names on one schema stay isolated. The Docker test fails, rather than skipping, on GitHub Actions when no runtime answers.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261005c/store/adversary-1/`: logs, plus `probe/` (certs, configs, scripts).
- `~/.cache/b10x-target/cortex-w2-store`: the assigned build dir.
- `~/.cache/claude-tmp/cortex-e2e*`: tempdirs the test harness creates and removes on drop.
- Docker containers `cortex-adv1-probe-pg`, `cortex-store-adversary-*` and `cortex-store-backend-2360331-*`: all removed, and `docker ps -a` shows none left.
- Lease `adversary-1-w20261005c-store`: acquired and released.

**7. Findings block**

```findings
[
  {"file": "src/main.rs", "line": 386, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "create seeds a postgres store without checking for an existing lineage, so a same-named instance from another home silently shares the first home's tenant and history"},
  {"file": "src/instance.rs", "line": 81, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "store_handle ignores the schema's sqlite value.path and reports created with the store in instances/<name>/store.sqlite"},
  {"file": "src/main.rs", "line": 571, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "update accepts a spec naming another SQLite file because the store comparison uses handles that drop the sqlite path"},
  {"file": "src/main.rs", "line": 356, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the relative-config refusal echoes store.value.config verbatim, printing a connection string and its password to stdout"},
  {"file": "tests/store_backend.rs", "line": 132, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the PostgreSQL container is removed only by Drop, so a terminated test run leaves it running"},
  {"file": "src/main.rs", "line": 450, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "seed-refused deletes the instance directory but leaves the postgres lineage, contradicting 'no store was kept' and making a retry with a different seed fail with AlreadySeeded"}
]
```
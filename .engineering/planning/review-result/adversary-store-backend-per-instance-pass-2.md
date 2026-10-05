---
format: aep.planning-md/3
id: review-result:adversary-store-backend-per-instance-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: store-backend-per-instance (wave 20261005c)'
relations:
- reviews: story:store-backend-per-instance
revision: 1
---
```
unit: U1 story:store-backend-per-instance, uncommitted working tree cortex-w2-store on base 6352006 (pass 2)
verdict: NEEDS-CHANGE
cases: executed 59→61, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (~/.cache/cortex-wave-20261005c/store/adversary-2/) plus the assigned build dir; details in part 6
needs-coordinator: none
```

**Verdict:** five of the six pass-1 findings are fixed with no regression. F1 is only partly fixed: two homes that create the same instance at the same moment are still both reported `created` (case red). I also found one smaller gap in the new update refusal.

**1. Diff stat**

`git --no-pager diff --stat` is the same as what I was handed: ekr.rs 117, instance.rs 11, main.rs 155, schedule.rs 12, spec_forms.rs 27. My only edit is an "ADVERSARY CASES" section appended to the end of the untracked `tests/store_backend.rs`. I touched no implementation file.

**2. Cases** (each run alone first)

| case | asserts | red |
|---|---|---|
| `adversary_two_homes_racing_for_one_postgres_tenant_are_not_both_created` | Home B's `ekr` is a wrapper. After B's `ekr head` answers "no seed", it lets home A create `pg` on the same schema, then B carries on. B must not report `created`. | `red-race.log`, EXIT=101, at :1056: `both homes report created for tenant "pg"`, `left: (Some(0), Some("created"))` |
| `adversary_an_update_does_not_freeze_a_refused_store_when_the_frozen_spec_is_unreadable` | A SQLite instance whose `instance.yaml` was deleted, updated to `{backend: postgres, value: {config: pg.json}}`, is refused | `red-update-unreadable.log`, EXIT=101, at :1088: `exit Some(0) … "outcome":"updated"`, and the relative config is now in the frozen spec |

**3. Suite** (run after both cases existed)
- `task check` → `test result: FAILED. 13 passed; 2 failed` (store_backend), `task: Failed to run task "test": exit status 101`, `EXIT=201` (`suite.log`).
- `cargo test --locked --workspace --no-fail-fast` → 61 executed, the only failures are my 2 cases, `EXIT=101` (`suite-no-fail-fast.log`). The 59 comes from fix-1's `gate-2.log`.

**4. Findings** (working tree above)

| # | file:line | verdict / origin | what was measured | what reaches it |
|---|---|---|---|---|
| A1 | src/main.rs:388 | NEEDS-CHANGE / introduced | Case 1 is red. The tenant check (`ekr head`) and the seed are two separate calls with nothing held between them. Seeding an identical seed again returns the first seed's event with exit 0 (pass-1 `tenant.log`: same `event_id`), so the second home goes through. | Pass-1 F1's scenario (two homes, one schema, the same name) with the two creates overlapping, e.g. two hosts set up by one config run. Possible fixes: make the seed unique per create so a racer gets `AlreadySeeded`, or refuse when the seed result's `committed_at` is older than the call. |
| A2 | src/main.rs:639 | INFEASIBLE / introduced | Case 2 is red. When the frozen spec can't be read, `Err(_) => false` throws away the `store_refusal` reason. | A registered instance whose `instance.yaml` is missing or does not parse. I built that state myself; nothing I found produces it. Fix: `Err(_) => reason.is_some()`. |

**5. Attacked and could not break**
- **F1, the error texts:** anything other than "no seed" (an unprovisioned schema, connection lost) is an `Err`, so the create is refused. The check fails safe. A substring false-positive would need a config path that contains the phrase; it's contrived, so I did not report it.
- **F2, F3:** both green. The update reason names only the field. The changed spec_forms case matches the coordinator's decision; its reason is asserted in store_backend.rs.
- **F4:** every refusal string in `store_refusal` and update is a constant. The `ekr head failed:` path repeats ekr's stderr, which pass-1 showed is redacted.
- **F5:** the Docker filter matches the label exactly (`probe-sweep-filter.log`: `store-backend-x` and a prefixed key were not matched). The sweep removes only containers carrying this file's label. One with no `created` label counts as stale, but it still carries the file's label.
- **F6, scratch SQLite:** `ekr` writes only `seed-check.sqlite` (`probe-files/`), and that file is removed on both the success and the failure path. The check uses the same `ekr`, host/tenant, seed and schema as the real seed. A failure that only PostgreSQL produces lands in the `seeded()` check after it (:406). I could not find a seed that SQLite accepts and PostgreSQL refuses.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261005c/store/adversary-2/`: logs, `probe-files/`, `tmp/` (now empty).
- `~/.cache/b10x-target/cortex-w2-store`: the assigned build dir.
- Docker: `cortex-adv2-*` probe containers and the test's labelled containers. All are removed, and the filter now lists none.
- I did not acquire a worktree session lease. That departs from the procedure, and there is no lease to release.

**7. Findings**

```findings
[
  {"file": "src/main.rs", "line": 388, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the tenant check and the seed are separate ekr calls and an identical re-seed exits 0, so two homes creating one postgres instance at once are both reported created on one tenant"},
  {"file": "src/main.rs", "line": 639, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "update discards the store_refusal reason when the frozen spec cannot be read, freezing a relative postgres config that create refuses"}
]
```
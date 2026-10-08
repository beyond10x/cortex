---
format: aep.planning-md/3
id: review-result:adversary-seen-documents-modelled-pass-2
kind: review-result
status: archived
title: 'Adversary pass 2: seen-documents-modelled (wave 20261005c)'
relations:
- reviews: story:seen-documents-modelled
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
unit: U2 story:seen-documents-modelled, uncommitted working tree on base 6352006 (cortex-w2-seen)
verdict: NEEDS-CHANGE
cases: executed 57→67, red 6
origin: introduced 4 / pre-existing 0 / undecided 2
wrote-outside-worktree: 10 paths under ~/.cache/cortex-wave-20261005c/seen/adversary-2/, plus build output in the assigned ~/.cache/b10x-target/cortex-w2-seen
needs-coordinator: decide whether the 2 symlink cases (origin undecided; one is in `src/instance.rs`, which this diff does not touch) stay in this unit or go to their own story. `task check` now stops at `test`, so `docs-check` did not run. I took no worktree lease.

**1. Diff.** `git --no-pager diff --stat` reads `112 files changed, 2041 insertions(+), 1829 deletions(-)`, the same as before I started. I added one untracked file, `tests/adversary_seen.rs` (322 lines). I changed no tracked file and no implementation file. The file was formatted with `rustfmt` on that file alone, which left `tests/common/` unchanged.

**2. Cases.** All are in `~/.local/state/worktree/trees/b10x/cortex/cortex-w2-seen/tests/adversary_seen.rs`, headed "ADVERSARY CASES". Each red case was run alone first; the output below was captured then, so its line numbers come from before `rustfmt`.

| case | asserts | now | red output when first run |
|---|---|---|---|
| `adv_an_update_respelling_the_instructions_path_through_a_missing_directory_keeps_runs_working` | after `update` with `instructions: nope/../prompt.md`, `cortex run t/news` still runs | red | `left: (1, Some("extraction-failed"))` ... `"reason":"instructions nope/../prompt.md: No such file or directory (os error 2)"` |
| `adv_an_update_making_the_instructions_path_absolute_keeps_runs_inside_the_instance` | after `update` with `instructions: /prompt.md`, runs stay inside the instance directory | red | `update {..."outcome":"updated"} pointed the instance's runs outside it: {..."reason":"instructions /prompt.md: No such file or directory (os error 2)"...}` |
| `adv_update_does_not_freeze_a_seed_path_create_refuses` | `create` refuses `nope/../docs` (checked: `seed-refused`), so `update` must not write it into the frozen spec | red | `update {..."outcome":"updated"} froze a path create refuses: ... seed: {documents: ["nope/../docs"]}` |
| `adv_the_list_row_carries_the_seed_digest_the_view_exposes` | the `cortex list` row carries the 64-hex `seed_digest` that the `Instances` view exposes | red | `the list row has no seed digest: {"description":"Test brain.","ekr_version":"0.0.30",...,"state":"Active",...}` |
| `adv_an_unchanged_symlinked_instructions_file_updates` | when `prompt.md` is a symlink, an unchanged update answers `updated` | red | `left: (1, Some("seed-change-refused"))  right: (0, Some("updated"))` |
| `adv_create_copies_a_symlinked_seed_directory` | `create` accepts a seed directory that is a symlink | red | `{"outcome":"seed-refused","detail":{"reason":".../docs/a.md: No such file or directory (os error 2)"}}` |
| 4 `adv_probe_*` (a reformatted spec, a changed instructions file, a `../outside` seed, a removed name, `registry.json`) | expected to hold | green | first run: `probes.log`, 4 passed |

My first symlink probe failed at `create`, not at the digest comparison it was written to test. I replaced it with the last two cases, which assert what they actually fail on.

**3. Suite run, after the cases existed**
- `task check`: `test result: FAILED. 4 passed; 6 failed` (adversary_seen), then `task: Failed to run task "check": task: Failed to run task "test": exit status 101`, `EXIT=201`. Validate, drift, fmt and clippy passed before it.
- `cargo test --locked --workspace --no-fail-fast`: 61 passed, 6 failed, `EXIT=101`. Every other binary is `ok` (lib 14, main 1, conformance 1, cost 8, e2e 9, seed_change 6, spec_compat 4, spec_forms 6, docs 8). The "before" count, 57, is the sum of the `test result` lines in fix-1/gate.log.

**4. Findings** (working tree on 6352006)

| # | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|
| 1 | `src/home.rs:95`: `normalise` treats `x/..` as nothing, by text alone. So `update` answers `updated` for a spec that names a path which does not exist, and writes that `..` path into the frozen spec, which `create`'s `freeze` refuses. Every later run then fails reading the instructions. The base compared the paths as written and refused. | `cortex update --spec`, with a path an operator could write. I have not shown that anyone writes one. | NEEDS-CHANGE | introduced |
| 2 | `src/home.rs:94`: `normalise` drops the leading `/`, so `/prompt.md` counts as the frozen `prompt.md`. The digest then reads `<spec dir>/prompt.md`, the update is applied, and runs read `/prompt.md` from the root filesystem: the frozen spec now points outside the instance directory. | the same path through `cortex update` | NEEDS-CHANGE | introduced |
| 3 | `src/main.rs:284`: `cortex list` leaves out `seed_digest`, which the unit's `Instances` view and the generated reference page both say it exposes. A naive fix would print `""`, because a loaded registry entry holds empty text (`src/home.rs:217`). | every `cortex list` | NEEDS-CHANGE | introduced |
| 4 | `src/home.rs:139`: the digest skips a symlinked seed file (the walk root is not followed, so `is_file()` is false). The frozen copy is a regular file, so every unchanged update is refused. By reading, the base `seed_bytes` had the same filter; I did not run it at the base. | an operator who symlinks a shared prompt file | CONFIRMED | undecided |
| 5 | `src/instance.rs:117`: `copy_tree` never creates the destination for a symlinked seed directory, so `create` answers `seed-refused`. This file is untouched by the diff; I did not run it at the base. | a seed directory that is a symlink | CONFIRMED | undecided |
| 6 | `src/main.rs:428`: `create` hashes the whole seed, and on refused paths it does so before `freeze` refuses them, which includes `../` and absolute paths. The result is never stored, because `to_json` drops it. No case: `/` is mounted `noatime`, so the reads cannot be observed. | every `create` | CONFIRMED (by reading) | introduced |

Suggested fixes (I applied none):
- **1 and 2:** before comparing digests, apply `freeze`'s path check (refuse `..` and absolute paths) to the update spec. Or limit `normalise` to `.` and repeated separators, and keep the leading `/`.
- **3:** either remove `seed_digest` from the view, or fill it from the frozen copy.
- **4:** follow the walk root when it is a symlink.

**5. Attacked and not broken**
- Pass-1 #1 and #2 are fixed: `seed_change.rs` covers both, and both are green.
- Pass-1 #3 is fixed: a renamed file and bytes moved between files are both caught.
- Pass-1 #4: the e2e test `a_refused_update_leaves_every_field_of_the_instance_unchanged` now checks the whole instance after a refused update.
- Pass-1 #5 is fixed; fix-1's mutation log shows the case going red when the mutation is applied.
- Pass-1 #7 is fixed: loading the registry reads no seed, checked by the unit test and by my probe.
- Pass-1 #8 is fixed, but more broadly than asked, which is what produced findings 1 and 2.
- `registry.json` never carries `seed_digest` after a create, an update and a refused update across two instances, and updating one instance leaves the other's entry unchanged. The generated `update_instance` reads only the addressed entry's digest, and no other generated behaviour reads it at all. So a stale or empty digest on another entry changes no outcome.
- A seed path pointing at an identical copy outside the spec directory is still refused as a seed change.
- A reformatted spec updates, and a changed instructions file is refused.
- A removed instance's name stays taken, with or without its directory.
- Pass-1 #6 (`excludes`) still stands as INFEASIBLE. I did not run it again, so it is not in the findings block.

**6. Paths written outside the worktree** (all under ~/.cache/cortex-wave-20261005c/seen/adversary-2/)
- `red-1-instructions.log`, `red-2-dotdot.log`, `red-3-list.log`, `red-4-symlink-dir.log`, `red-5-symlink-file.log`, `red-6-absolute.log`, `probes.log`, `gate.log`, `suite-nofailfast.log`
- `tmp/`, which is empty now
- build output in ~/.cache/b10x-target/cortex-w2-seen (assigned)

```findings
[
 {"file":"src/home.rs","line":95,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"lexical folding of x/.. lets update accept and freeze a seed or instructions path that create refuses and that does not resolve in the instance directory, so every later run fails"},
 {"file":"src/home.rs","line":94,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"normalise drops the leading slash, so update accepts an absolute instructions path as the frozen relative one and later runs read a file outside the instance directory"},
 {"file":"src/main.rs","line":284,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the Instances view and its generated reference promise seed_digest on list, cortex list omits it, and a loaded entry would carry empty text"},
 {"file":"src/home.rs","line":139,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"a symlinked seed or instructions file is skipped by the digest but frozen as a regular file, so an unchanged update is refused"},
 {"file":"src/instance.rs","line":117,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"copy_tree never creates the destination for a symlinked seed directory, so create answers seed-refused"},
 {"file":"src/main.rs","line":428,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"create hashes the whole seed, including paths outside the spec directory before freeze refuses them, for a digest that is never persisted"}
]
```
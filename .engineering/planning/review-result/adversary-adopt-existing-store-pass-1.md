---
format: aep.planning-md/3
id: review-result:adversary-adopt-existing-store-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: adopt-existing-store (wave 20261006a)'
relations:
- reviews: story:adopt-existing-store
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
The adopt unit is red: 5 new cases fail, and 1 of them is a blocker. When `--store` is a symlink to a store that another process holds open, `adopt` drops the newest revision and still reports `adopted`.

unit: U2 story:adopt-existing-store, uncommitted working tree on base 88b87fd (cortex-w8-adopt)
verdict: NEEDS-CHANGE
cases: executed 309→314, red 5
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 tree (scratch, see part 6)
needs-coordinator: my 5 cases sit in the untracked `tests/adopt.rs` (line 456 on), so `git diff --stat` cannot show them; I did not acquire a worktree session lease

**1. Diff stat.** `git --no-pager diff --stat` shows `122 files changed, 3968 insertions(+), 516 deletions(-)`. All of that is the implementor's change; I added nothing to it. My only edit is an appended section in `tests/adopt.rs`, a test file the implementor created and left untracked, so the stat does not list it. The section is headed `// ADVERSARY CASES` and runs from line 456 to 689. I ran `rustfmt` on that file only. No implementation file was touched.

**2. Cases added** (`tests/adopt.rs`). Each was run alone and was red the first time; logs are in `adversary-1/red-*.log`.

| line | asserts | red output |
|---|---|---|
| 500 | A store held open by another process, with revision 3 still in its `-wal`, adopted through a symlinked `--store`, reports `revision` 3 | `adopt lost the revisions in the -wal` … `"revision":2` … `left: Number(2) right: 3` |
| 531 | A store with a `-wal` and no `-shm` is copied without a file appearing beside it | `left: ["store.sqlite", "store.sqlite-shm", "store.sqlite-wal"] right: ["store.sqlite", "store.sqlite-wal"]` |
| 570 | A destination too small for the copy (`ulimit -f 64`) is not reported as `store-unreadable` | `{"reason":"cannot copy --store: disk I/O error"},"outcome":"store-unreadable"` |
| 600 | Two `--seen` files for one source are refused (exit 2) | `left: Some(0) right: Some(2)` … `"seen":["brain/notes","brain/notes"]` |
| 643 | A PostgreSQL lineage that is already adopted is not adopted again under another name. A stand-in `ekr` answers `ontology`. | `left: Some("adopted") right: Some("adopted")` |

**3. Suite run** (after the cases existed):
- `task check ESS=$HOME/.cache/ess/toolchains/0.52.0/ess` gave `EXIT=201`. It ran 134 cases and stopped at the first failing target: `test result: FAILED. 7 passed; 5 failed` (adopt).
- `cargo test --locked --workspace --no-fail-fast` gave `EXIT=101`, with 309 passed and 5 failed, all 5 in `-p cortex-cli --test adopt`. The 309 matches the implementor's `gate.log` total.

**4. Findings**

| file:line | finding | what reaches it | verdict | origin |
|---|---|---|---|---|
| src/instance.rs:214 | The `-wal` check looks beside the given path, not the resolved file. A symlinked `--store` is therefore opened `immutable=1`, and the revisions in the WAL are lost silently. | A file symlink given as `--store`, plus any reader holding the store open (`ekr view` or `mcp-http`). `operating.md` only says to stop writers, not readers. | NEEDS-CHANGE (blocker) | introduced |
| src/instance.rs:214 | The read-only fallback creates or rewrites `-shm` beside the source. The probe of a dead holder showed the `-shm` hash changing. This contradicts "writes nothing beside `from`". | A store whose writer was killed, or a copy that took only the database and its WAL | CONFIRMED (note) | introduced |
| src/main.rs:780 | There is no free-space check before the copy. The copy writes until the destination refuses, and that refusal is labelled `store-unreadable`. | Adopting a store larger than the free space under the cortex home | CONFIRMED (warning) | introduced |
| src/main.rs:816 | A second `--seen` for the same source silently replaces the first. | The operator repeats the flag | CONFIRMED (note) | introduced |
| src/main.rs:831 | Only the name is checked. A PostgreSQL lineage (same config and host) can become two instances, each with its own seen state. | Adopting the same store twice under two names. The probe adopted b1, b2 and b3 on one config. | CONFIRMED (warning) | introduced |
| tests/adopt.rs:251 | A `--seen` file naming a document the store lacks means that document is never extracted. The implementor's own test asserts exactly this (a.md is marked seen, the store has no evidence of it, `documents_applied: 1`). Adopt checks nothing and reports no counts. | Any `--seen` file from another home or pipeline | CONFIRMED (warning) | introduced |
| src/instance.rs:214 | Time-of-check gap: a writer that opens the store after the `-wal` check, during an `immutable` copy, can tear the copy. I did not reproduce this. | Nothing found | INFEASIBLE (note) | introduced |

Suggested fixes, not applied:
- Canonicalise `from` before deriving the `-wal` path, or drop `immutable` entirely.
- Map destination errors to a failure and check `statvfs` before copying.
- Refuse a duplicate `--seen`.
- Record the canonical PostgreSQL config and tenant per instance, and refuse a match.

**5. Attacked and could not break:**
- **Host mismatch:** a different profile, agent ids or agent names all get `bootstrap-authority-mismatch`, which is reported as `store-unreadable`.
- **Older stores:** stores from ekr 0.0.20, 0.0.25, 0.0.27 and 0.0.29 read fine under 0.0.30. No newer `ekr` binary is available locally to test the other direction.
- **Reported revision:** `ontology.revision` equals the head even when the ontology itself did not change.
- **Evidence:** evidence lives inside the database, with no files beside it.
- **PostgreSQL paths:** symlinked, relative and `..` forms of the config path are accepted, and a different file is refused.
- **Credentials:** ekr's PostgreSQL errors print no DSN or password in any of 7 malformed or unreachable configs.
- **Concurrency:** concurrent adopts are serialised by the lock on the cortex home.
- **Leftovers and collisions:** a leftover directory is refused, and name collisions are covered.

**6. Paths written outside the worktree:**
- `~/.cache/cortex-wave-20261006a/adopt/adversary-1/`: probes in `probe/p1` to `probe/p9` with their scripts, the `red-*.log` files, `suite.log`, `suite-nofailfast.log`, and `tmp/`.
- The build went into `~/.cache/b10x-target/cortex-w8-adopt` as assigned.

```findings
[
 {
  "file": "src/instance.rs",
  "line": 214,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "a symlinked --store held open by another process is opened immutable=1 and adopt reports revision 2 while the store's head is 3, dropping the revisions in its -wal"
 },
 {
  "file": "src/instance.rs",
  "line": 214,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the read-only fallback creates or rewrites store.sqlite-shm beside the source, contradicting the doc comment's \"writes nothing beside from\""
 },
 {
  "file": "src/main.rs",
  "line": 780,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "no free-space check precedes the copy, and a destination that cannot hold it is reported as store-unreadable"
 },
 {
  "file": "src/main.rs",
  "line": 816,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a second --seen for the same source silently replaces the first"
 },
 {
  "file": "src/main.rs",
  "line": 831,
  "category": "concurrency",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "one PostgreSQL lineage can be adopted as several instances because only the name is checked"
 },
 {
  "file": "tests/adopt.rs",
  "line": 251,
  "category": "judgement",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a --seen document the store lacks is never extracted, and the unit's own test asserts that outcome"
 },
 {
  "file": "src/instance.rs",
  "line": 214,
  "category": "concurrency",
  "severity": "note",
  "verdict": "INFEASIBLE",
  "origin": "introduced",
  "message": "a writer that opens the store after the -wal check can tear an immutable copy; not reproduced"
 }
]
```
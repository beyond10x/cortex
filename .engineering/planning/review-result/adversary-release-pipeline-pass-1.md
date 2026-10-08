---
format: aep.planning-md/3
id: review-result:adversary-release-pipeline-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: release-pipeline (wave 20261005a)'
relations:
- reviews: story:release-pipeline
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
unit: story:release-pipeline (U2, wave 20261005a), uncommitted working tree on base fe85624
verdict: red
cases: executed 31→37, red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: ~/.cache/cortex-wave-20261005a/release/adversary-1/ (all of it, listed in part 6)
needs-coordinator: none

The release workflow is sound on what matters most: the tag check, permissions, injection, artifact layout and checksum format all held. I found 1 warning and 4 notes, none a blocker. The warning is that the release tarball is not reproducible: packaging the same binary twice gives two different checksums.

**1. `git --no-pager diff --stat`** (the tree is as I was handed it; I wrote nothing in it)
```
 AGENTS.md | 43 +++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 43 insertions(+)
```
`release.yml` and `CHANGELOG.md` are untracked, as handed over.

**2. Cases added**, all in `adversary-1/attack.sh`. Each one runs the workflow's own `run:` scripts, read out of `release.yml`. Each was run alone before the suite, and each was red.

| case | what it asserts | now | red output (`red-<case>.log`) |
|---|---|---|---|
| repro | packaging one binary twice, 2 s apart, gives the same `SHA256SUMS` | red | `run A: e09e25ce…` / `run B: 9429e6b6…` / `FAIL repro: two packagings of one binary give one SHA256SUMS` |
| owner | every tar entry is uid/gid 0/0 | red | `owners (uid/gid): 1000/1000` / `FAIL owner: every entry is 0/0` |
| pins | each pin comment is a tag that resolves to the pinned SHA (checked with `git ls-remote`) | red, 1 of 4 | `FAIL pins: dtolnay/rust-toolchain@02cb101e… is tag master (tag resolves to '<no such tag>')` |

Why `repro` differs:
- The gzip header mtime is 0 in both runs, so gzip is not the cause.
- The tar entries carry the copy time from `cp` (13:20:08 vs 13:20:11).
- The tar entries also carry the build user's owner and group.

**3. Suite run** (after the cases existed): `check-release.sh <tree> adversary-1` followed by `attack.sh <tree> adversary-1`, written to `suite.log`.
```
RESULT: 0 failed
EXIT=0
FAIL repro: two packagings of one binary give one SHA256SUMS
FAIL owner: every entry is 0/0
FAIL pins: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de is tag master (tag resolves to '<no such tag>')
RESULT: ran 6, 3 failed
EXIT=1
```
The `before` count of 31 comes from the implementor's `green.log`. The run gave 37 = 31 + 6.

**4. Findings**

| file:line | verdict / origin | finding | what reaches it |
|---|---|---|---|
| .github/workflows/release.yml:80 | NEEDS-CHANGE / introduced, warning | The tarball contents depend on the time and the user that packaged it. A re-run or an independent rebuild cannot be checked against the published `SHA256SUMS`. Fix: `tar --sort=name --mtime=@<commit time> --owner=0 --group=0 --numeric-owner`, piped into `gzip -n`. | Every release run, and the "re-run the workflow" recovery path in AGENTS.md |
| .github/workflows/release.yml:80 | CONFIRMED / introduced, note | Tar entries record the build user's uid/gid (1000 here; on CI this would be the runner's account, inferred). | Every release run |
| .github/workflows/release.yml:38 | CONFIRMED / introduced, note | The comment `# master` names a branch, not a tag, which is not what the brief asked for. The SHA is 2 commits behind master, and the action's only tag, `v1`, points at 7e38f4b. The SHA itself reuses check.yml's pin, as the brief required. The implementor's check accepts any comment word, so it cannot catch this. | Anyone reading the pin comment |
| AGENTS.md:154 | CONFIRMED / introduced, note | The procedure says to tag a commit on `origin/main`, but nothing in the workflow checks this. Any published release whose tag matches `Cargo.toml` gets assets attached, including a prerelease or a tag on another branch. | Anyone with permission to create releases (no case written; this is a judgement) |
| CHANGELOG.md:8 | CONFIRMED / introduced, note | Following AGENTS.md:148 (rename `## Unreleased` to the version heading) carries "No version of cortex has been released yet." into the released section. | The first release cut by the documented procedure |

**5. Attacked and not broken**
- **Tag/version mismatch:** a mismatch fails before upload, and so does a tag without the `v` prefix.
- **Injection:** no `${{ }}` appears inside any `run:` script. The tag and version reach the scripts only through `env:`.
- **Permissions:** the top level and the build job have `contents: read`; only the publish job has `contents: write`, and it runs only on `release`.
- **Draft releases:** `published` does not fire on creating a draft. A tag push alone starts nothing.
- **Pins:** checkout v7, upload-artifact v7.0.1 and download-artifact v8.0.1 match upstream.
- **SHA256SUMS:** two-space `sha256sum` format, same as the ess/aep convention; it is checked again in the publish job.
- **Artifact contents:** binary, `LICENSE`, `README.md`; the packed binary prints `cortex 0.1.0`.
- **AGENTS.md vs the workflow:** the triggers, asset names, `--clobber` behaviour and verification commands match.
- **CHANGELOG claims:** the scenario count (35), the "two failures disable the timer" rule (`src/ports.rs:175`), tavily as the default, and revalidation all match the code and README.
- **Gap, not a break:** actionlint's shellcheck rule was disabled in `actionlint.log` because shellcheck is not installed. I reviewed the shell by eye only.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261005a/release/adversary-1/attack.sh`
- `…/adversary-1/red-repro.log`, `red-owner.log`, `red-pins.log`, `suite.log`
- `…/adversary-1/runA/`, `runB/`, `runO/`, `runner/` (packaging output)
- `…/adversary-1/tools`: a symlink to `../tools`

I read the existing release binary in `~/.cache/b10x-target/cortex-w1-release` but built nothing.

**7. Findings**
```findings
[
{"file":".github/workflows/release.yml","line":80,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Packaging the same binary twice yields different SHA256SUMS because tar records cp-time mtimes and the build user, so a re-run or independent rebuild cannot be verified against the published checksum."},
{"file":".github/workflows/release.yml","line":80,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Tarball entries carry the packaging host's uid/gid instead of 0/0."},
{"file":".github/workflows/release.yml","line":38,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The pin comment '# master' names a branch, not a tag, and the pinned SHA is not master's head; the brief requires a tag comment."},
{"file":"AGENTS.md","line":154,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"AGENTS.md requires the tag on an origin/main commit but the workflow attaches assets to any published release whose tag matches Cargo.toml, including prereleases and tags off main."},
{"file":"CHANGELOG.md","line":8,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Renaming '## Unreleased' per AGENTS.md:148 carries 'No version of cortex has been released yet.' into the released version's section."}
]
```

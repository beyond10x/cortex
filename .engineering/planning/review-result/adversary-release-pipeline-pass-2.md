---
format: aep.planning-md/3
id: review-result:adversary-release-pipeline-pass-2
kind: review-result
status: archived
title: 'Adversary pass 2: release-pipeline (wave 20261005a)'
relations:
- reviews: story:release-pipeline
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
unit: U2 story:release-pipeline, pass 2, uncommitted working tree on fe85624 (AGENTS.md diff, untracked release.yml and CHANGELOG.md)
verdict: CONFIRMED (3 notes; no blocker and no warning)
cases: executed 48→71, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: ~/.cache/cortex-wave-20261005a/release/adversary-2/
needs-coordinator: none

The 5 pass-1 findings are fixed and nothing regressed. There are 3 new notes.

On Q3, `actions/checkout` at 3d3c42e5 with fetch-depth 0 does create refs/remotes/origin/main:
- src/git-source-provider.ts:188-194 takes depth ≤ 0 to getRefSpecForAllHistory.
- src/ref-helper.ts:69-70 then fetches `+refs/heads/*:refs/remotes/origin/*` and `+refs/tags/*:refs/tags/*`.
- The same refspecs are at dist/index.js:41506 and action.yml:75.

## Cases (adversary-2/attack.sh)

| case | asserts | now |
|---|---|---|
| umask | umask 077, 002 and 027 give the same SHA256SUMS as 022 | green |
| paths | another RUNNER_TEMP, and a cwd reached through a symlink, give the same SHA256SUMS | green |
| locale | `C` and `en_US.utf8` give the same SHA256SUMS | green |
| setgid | a RUNNER_TEMP below a setgid directory gives the same SHA256SUMS | red: the directory entry is `drwxr-sr-x` |
| hostpath | the packed binary names no path of the build host | red: 124 `$CARGO_HOME/registry` strings |
| guard | 14 checks in a clone fetched the way checkout@3d3c42e5 fetches | green |
| noorigin | a depth-1 layout with no origin/main is refused | green |

## Suite run

| command | result |
|---|---|
| `check-release.sh` | 0 failed, EXIT=0 |
| `attack.sh` | ran 23, 2 failed (setgid, hostpath), EXIT=1 |

## Findings

| file:line | verdict / origin | finding |
|---|---|---|
| AGENTS.md:144 | CONFIRMED / introduced, note | "The tarball is reproducible" holds only for repackaging the same binary. The binary embeds 124 registry paths of the build host, so an independent rebuild gives a different checksum. |
| release.yml:82 | INFEASIBLE / introduced, note | `--mode='u=rwX,go=rX'` keeps the setgid bit. Adding `a-s` fixes it; tested on a copy. |
| CHANGELOG.md:10 | CONFIRMED / introduced, note | The list names `source enable` but leaves out `source add`. |

## Attacked, not broken

- Pin exception: sound. It requires master to contain the SHA, and it fails closed.
- Guard order: guard → download → upload.
- Guard fails closed on a missing origin/main, a missing tag, a tree tag, and every prerelease value other than `false`.
- No injection.
- `^{commit}` peels lightweight, annotated and nested tags.
- A `+` in the tag name passes.

```findings
[
{"file":"AGENTS.md","line":144,"category":"property","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"'The tarball is reproducible' holds only for repackaging the same binary; the binary embeds 124 builder $CARGO_HOME paths, so an independent rebuild yields another checksum."},
{"file":".github/workflows/release.yml","line":82,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"--mode='u=rwX,go=rX' keeps the setgid bit, so packaging below a setgid directory records drwxr-sr-x and changes SHA256SUMS; adding a-s fixes it."},
{"file":"CHANGELOG.md","line":10,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The command list names 'source enable' but omits the user-facing 'source add' that cortex source --help lists."}
]
```

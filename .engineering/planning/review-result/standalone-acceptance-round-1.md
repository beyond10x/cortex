---
format: aep.planning-md/3
id: review-result:standalone-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, standalone 1.0, round 1
relations:
- reviews: epic:standalone-1-0
- reviews: story:spec-standalone-types
- reviews: story:store-backend-per-instance
- reviews: story:redaction-before-model
- reviews: story:structured-source
- reviews: story:codex-model-backend
- reviews: story:run-snapshots
- reviews: story:adopt-existing-store
- reviews: story:release-pipeline
- reviews: story:docs-for-1-0
- reviews: story:release-1-0
revision: 1
---
needs-revision
story:spec-standalone-types — the acceptance joins three independent outcomes (`task check` passes, an unchanged `examples/example.yaml` behaves as on `main`, five non-default settings are refused), so one can pass while another fails — .engineering/planning/story/spec-standalone-types.md:57
story:store-backend-per-instance — the test is "skipped with a named reason when no container runtime exists", so the acceptance is met by a skipped test and names no environment where it must actually run — .engineering/planning/story/store-backend-per-instance.md:38
story:codex-model-backend — "the stand-in receives the schema and the prompt on its documented interface" names no interface, so the check is whatever the story's own stand-in accepts; a second sentence also adds an independent commit-message requirement — .engineering/planning/story/codex-model-backend.md:33
story:run-snapshots — the acceptance joins two independent outcomes (restore returns the earlier head; `snapshots.keep: 2` leaves 2 snapshots) and checks none of the Work's refusals (lock held, `postgres`) or the `RestoreSnapshot` marker outcomes — .engineering/planning/story/run-snapshots.md:48
story:adopt-existing-store — "a `cortex run` applies on top of it" names no observable (revision advanced? which documents applied?), and the acceptance exercises none of the outcomes the Work lists (name taken, store unreadable, backend mismatch) or the seen-documents rule — .engineering/planning/story/adopt-existing-store.md:48
story:docs-for-1-0 — "the deployed site shows the new pages" names no page or URL, and the scope edits only existing files, so it reads the same before and after the work; the acceptance also joins three independent checks (review table, `task website`/`task check`, deployed site) — .engineering/planning/story/docs-for-1-0.md:45
story:release-1-0 — "completes the site's quickstart ... with the exit statuses recorded" names no expected status and no place they are recorded, so any recording passes — .engineering/planning/story/release-1-0.md:34

What I read: 10 story ids plus `epic:standalone-1-0` and the `spec/domains/instance.yaml` diff, via `aep plan artifact show`, `kinds` and `lifecycle story`. I also ran `git diff` on the spec, `git tag`, `ls tests` and `grep` on `Cargo.toml`.
Approved on acceptance: `story:redaction-before-model`, `story:structured-source` and `story:release-pipeline` are one observable scenario each, with a stated before and after.
What I could not establish: whether the `plan/standalone` branch and `task check` drift check pass, since I ran neither. The stories' `depends_on` and run-order edges are outside my lane (design and parallel-safety). They did not affect the verdict.

```findings
[
  {"file": ".engineering/planning/story/spec-standalone-types.md", "line": 57, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins three independent outcomes (`task check` passes, an unchanged `examples/example.yaml` behaves as on `main`, five non-default settings are refused), so one can pass while another fails"},
  {"file": ".engineering/planning/story/store-backend-per-instance.md", "line": 38, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the test is \"skipped with a named reason when no container runtime exists\", so the acceptance is met by a skipped test and names no environment where it must actually run"},
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 33, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"the stand-in receives the schema and the prompt on its documented interface\" names no interface, so the check is whatever the story's own stand-in accepts; a second sentence also adds an independent commit-message requirement"},
  {"file": ".engineering/planning/story/run-snapshots.md", "line": 48, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins two independent outcomes (restore returns the earlier head; `snapshots.keep: 2` leaves 2 snapshots) and checks none of the Work's refusals (lock held, `postgres`) or the `RestoreSnapshot` marker outcomes"},
  {"file": ".engineering/planning/story/adopt-existing-store.md", "line": 48, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"a `cortex run` applies on top of it\" names no observable (revision advanced? which documents applied?), and the acceptance exercises none of the outcomes the Work lists (name taken, store unreadable, backend mismatch) or the seen-documents rule"},
  {"file": ".engineering/planning/story/docs-for-1-0.md", "line": 45, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"the deployed site shows the new pages\" names no page or URL, and the scope edits only existing files, so it reads the same before and after the work; the acceptance also joins three independent checks (review table, `task website`/`task check`, deployed site)"},
  {"file": ".engineering/planning/story/release-1-0.md", "line": 34, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"completes the site's quickstart ... with the exit statuses recorded\" names no expected status and no place they are recorded, so any recording passes"}
]
```

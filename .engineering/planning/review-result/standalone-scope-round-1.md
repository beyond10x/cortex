---
format: aep.planning-md/3
id: review-result:standalone-scope-round-1
kind: review-result
status: archived
title: Scope critic, standalone 1.0, round 1
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
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:39Z", actor: "human:timo", revision: 2}
---
needs-revision
epic:standalone-1-0 — "anyone can install a released cortex … and read the documentation that says all of it" is promised, but no item documents installing the released binary: `story:docs-for-1-0` scopes five files and omits `website/docs/quickstart.md`, whose step 1 is still `git clone` plus `cargo install --locked --path .`. `story:docs-for-1-0` is the natural owner — .engineering/planning/epic/standalone-1-0.md:13
story:release-pipeline — "Cut `v0.2.0` with it, to prove the process before 1.0" publishes a tag and GitHub Release that no sentence in the parent asks for; the epic promises only a 1.0 release and calls this story "independent" — .engineering/planning/story/release-pipeline.md:35

**What you read:** 12 artifacts (the epic, the ten stories, the vision), plus `story:timer-runs-unattended`, `story:seen-documents-modelled` and `epic:first-web-instance` for coverage elsewhere. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `kinds`, `relations`, and `grep` on `website/docs`.

**Promises extracted from the parent and traced:**
- Extracted: 9. That is 8 outcome promises plus the "Decided" backend choice, and the 7 "Stories and order" entries are checked separately.
- Traced: 8 of 9, with one partial.
- Backend choice, redaction, recovery, adopt, structured source, Codex and the types are each claimed once.
- Documentation of features is claimed by `story:docs-for-1-0`.
- Installing a released cortex is claimed by `story:release-pipeline` and `story:release-1-0`. The documentation of that install is untraced, which is the first finding.
- Web, Connectors and files sources already exist and are covered by `epic:first-web-instance`, so they are not gaps.
- All 7 order entries trace to a story.
- Nothing lands in the "Not in this epic" exclusions.

**What I could not establish:**
- "No feature depends on one company's setup" is a constraint, not a claim. I found no item that contradicts it.
- Out of my lane, and not counted in the verdict:
  - `story:release-1-0` depends on `story:timer-runs-unattended`, which belongs to the other epic and is not in this epic's order (design lane).
  - `story:release-1-0`'s acceptance runs "the site's quickstart" with the released binary, but that quickstart builds from source (acceptance lane).
  - `story:adopt-existing-store` and `story:run-snapshots` test SQLite only, although their work text mentions Postgres (acceptance lane).

```findings
[
  {"file": ".engineering/planning/epic/standalone-1-0.md", "line": 13, "category": "scope", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"anyone can install a released cortex … and read the documentation that says all of it\" is promised, but no item documents installing the released binary: story:docs-for-1-0 scopes five files and omits website/docs/quickstart.md, whose step 1 is still git clone plus cargo install --path (website/docs/quickstart.md:19-24); story:docs-for-1-0 is the natural owner"},
  {"file": ".engineering/planning/story/release-pipeline.md", "line": 35, "category": "scope", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "\"Cut v0.2.0 with it, to prove the process before 1.0\" publishes a tag and GitHub Release that no sentence in the parent asks for; the epic promises only a 1.0 release and calls this story independent"}
]
```

---
format: aep.planning-md/3
id: review-result:parallel-safety-round-1
kind: review-result
status: archived
title: Parallel-safety critic, round 1
relations:
- reviews: epic:first-web-instance
- reviews: story:ci-runs-task-check
- reviews: story:seen-documents-modelled
- reviews: story:extraction-links-facts
- reviews: story:timer-runs-unattended
- reviews: story:first-web-instance
- reviews: story:web-pages-cited-as-urls
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:38Z", actor: "human:timo", revision: 2}
---
needs-revision
story:web-pages-cited-as-urls — its scope item "the EKR pin" is unplaced, and the pin sits in files that `story:ci-runs-task-check` will hard-code (`.github/workflows/check.yml`, a file that does not exist yet) and in `AGENTS.md`, `tests/e2e.rs` and `src/main.rs`; the body should name the pin's files and either order after the CI story or leave the workflow's pin out of the bump (inferred, the pin files are cited in the tree) — .engineering/planning/story/web-pages-cited-as-urls.md:28
story:web-pages-cited-as-urls — its scope omits `src/extract.rs`, which calls `evidence::identity` (line 138), cites `issued.id` in `web_page_facts` (line 311) and builds the evidence list (line 350), while `story:extraction-links-facts` edits that same file, and neither body says so (inferred, from the tree) — .engineering/planning/story/web-pages-cited-as-urls.md:28
story:timer-runs-unattended — its acceptance (`runs`, `consecutive_failures`, "no new document makes no model call") is implemented in `src/state.rs`, `src/run.rs:128-232`, `src/ports.rs:175,260` and `src/home.rs:172`, yet its scope lists only `src/schedule.rs` and `README.md`; any fix those checks force lands on files `story:seen-documents-modelled` claims (`src/state.rs`, `src/home.rs`), and neither body says so (inferred, from the tree) — .engineering/planning/story/timer-runs-unattended.md:33
story:seen-documents-modelled — it changes the seen-state code in `src/state.rs` (format `cortex.seen/1`, line 43) and regenerates the model crate, while `story:first-web-instance` runs the real instance for 7 days with no edge ordering after this story, so a format or generated-type change can land under a running instance (inferred, from the tree) — .engineering/planning/story/seen-documents-modelled.md:33

Both remedies for each pair are open: add an ordering edge that records the shared surface as its reason, or split the surface so the two no longer share it. I choose neither.

**What I read:** 6 stories, using `aep plan artifact show` on each, `aep plan artifact waves`, `aep plan artifact graph`, and `git grep` and reads of `src/`, `tests/`, `examples/`, `Taskfile.yml` and `AGENTS.md`.

**Surfaces:** 4 stories have a cited scope (`story:ci-runs-task-check`, `story:seen-documents-modelled`, `story:extraction-links-facts`, `story:first-web-instance`). 2 are cited but too narrow for their acceptance and were widened by inference (`story:timer-runs-unattended`, `story:web-pages-cited-as-urls`). 0 could not be placed. `aep plan artifact waves` reports 0 waves and 6 unassessed because no story has a recorded scope.

**Not established:**
- Whether `src/schedule.rs` or `README.md` is touched by any story other than `story:timer-runs-unattended`: it is not.
- The pairs `story:ci-runs-task-check` / `story:timer-runs-unattended` and `story:web-pages-cited-as-urls` / `upstream-blocker:ekr-url-evidence` share no files; the first is ordered by an edge and the second by a blocker.
- Out of my lane: `story:ci-runs-task-check` has no file in common with `story:timer-runs-unattended`, so the edge between them is not justified by a shared surface. That is a design question.
- Out of my lane: `story:web-pages-cited-as-urls` cannot start until `upstream-blocker:ekr-url-evidence` closes, so it is never worked concurrently now.

```findings
[
  {
    "file": ".engineering/planning/story/web-pages-cited-as-urls.md",
    "line": 28,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "its scope item \"the EKR pin\" is unplaced, and the pin sits in files that story:ci-runs-task-check will hard-code (.github/workflows/check.yml, a file that does not exist yet) and in AGENTS.md, tests/e2e.rs and src/main.rs; the body should name the pin's files and either order after the CI story or leave the workflow's pin out of the bump (inferred, the pin files are cited in the tree)"
  },
  {
    "file": ".engineering/planning/story/web-pages-cited-as-urls.md",
    "line": 28,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "its scope omits src/extract.rs, which calls evidence::identity (line 138), cites issued.id in web_page_facts (line 311) and builds the evidence list (line 350), while story:extraction-links-facts edits that same file, and neither body says so (inferred, from the tree)"
  },
  {
    "file": ".engineering/planning/story/timer-runs-unattended.md",
    "line": 33,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "its acceptance (runs, consecutive_failures, no new document makes no model call) is implemented in src/state.rs, src/run.rs:128-232, src/ports.rs:175,260 and src/home.rs:172, yet its scope lists only src/schedule.rs and README.md; any fix those checks force lands on files story:seen-documents-modelled claims (src/state.rs, src/home.rs), and neither body says so (inferred, from the tree)"
  },
  {
    "file": ".engineering/planning/story/seen-documents-modelled.md",
    "line": 33,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "it changes the seen-state code in src/state.rs (format cortex.seen/1, line 43) and regenerates the model crate, while story:first-web-instance runs the real instance for 7 days with no edge ordering after this story, so a format or generated-type change can land under a running instance (inferred, from the tree)"
  }
]
```

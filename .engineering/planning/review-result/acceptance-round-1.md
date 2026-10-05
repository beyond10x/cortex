---
format: aep.planning-md/3
id: review-result:acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1
relations:
- reviews: epic:first-web-instance
- reviews: story:ci-runs-task-check
- reviews: story:seen-documents-modelled
- reviews: story:extraction-links-facts
- reviews: story:timer-runs-unattended
- reviews: story:first-web-instance
- reviews: story:web-pages-cited-as-urls
revision: 1
---
needs-revision
story:timer-runs-unattended — the acceptance joins four independent outcomes with semicolons (journal completeness, `runs` advancing, no model call on an unchanged run, failure counter raised then reset), so one can fail while the others pass and the story is neither done nor not done; the body should state one checkable outcome, or move the others to separate stories — .engineering/planning/story/timer-runs-unattended.md:29
story:first-web-instance — "three questions answered through `ekr mcp` `search`" names no questions and no pass rule, so whoever picks the questions after the 7 days decides whether the acceptance is met; the body should name the three questions, or say what makes an answer correct, and give "the store holds edges" a count against the zero baseline — .engineering/planning/story/first-web-instance.md:31

What I read: 8 of 8 ids (epic:first-web-instance, story:ci-runs-task-check, story:seen-documents-modelled, story:extraction-links-facts, story:timer-runs-unattended, story:first-web-instance, story:web-pages-cited-as-urls, upstream-blocker:ekr-url-evidence), with `aep plan artifact show <id>` for each. I also ran `aep plan artifact kinds` and `aep plan artifact lifecycle story`, and grepped the tree. The tree matches the drafts on these points:
- `spec/domains/instance.yaml` carries three `UNMAPPED` markers, at lines 213, 328 and 382.
- `cortex-cli` is the real package name.
- `runs` and `consecutive_failures` exist in `src/home.rs`.
- `~/.cache/cortex-smoke-8c9138d1` exists.
- `examples/seed/schema.yaml` exists.

What I could not establish:
- Whether the smoke run's 8 documents were saved anywhere the benchmark can use. The story's Work section says to save them, so I did not count it as a defect.
- The epic has no `## Acceptance` heading. Its `## Outcome` is observable, and the stories carry the checks, so I did not flag it.

Out of my lane (not counted toward the verdict):
- `src/ports.rs:175` computes `disable = consecutive_failures >= 1`. That looks inconsistent with the failure-then-reset behaviour in the timer story, which is a design or coverage question.
- `upstream-blocker:ekr-url-evidence` says "Filed upstream" but the text says the upstream story is only "drafted".

```findings
[
  {
    "file": ".engineering/planning/story/timer-runs-unattended.md",
    "line": 29,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the acceptance joins four independent outcomes (journal completeness, runs advancing, no model call on an unchanged run, failure counter raised then reset), so one can fail while the others pass and the story is neither done nor not done"
  },
  {
    "file": ".engineering/planning/story/first-web-instance.md",
    "line": 31,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"three questions answered through ekr mcp search\" names no questions and no pass rule, and \"the store holds edges\" gives no count against the zero baseline, so whoever picks the questions after the 7 days decides whether the acceptance is met"
  }
]
```

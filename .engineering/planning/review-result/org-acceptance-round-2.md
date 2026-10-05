---
format: aep.planning-md/3
id: review-result:org-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, organisation scale, round 2
relations:
- reviews: epic:organisation-scale-instance
- reviews: story:connectors-source-walks
- reviews: story:file-records
- reviews: story:redaction-names-and-gate
- reviews: story:structured-from-files-and-drops
- reviews: story:document-time-as-valid-time
- reviews: story:run-gate
- reviews: story:quality-judge
- reviews: story:spec-standalone-types
revision: 1
---
needs-revision
story:connectors-source-walks — the window check feeds `{since}` from a state fixture, so nothing observes that a run records its start instant or that the first-run default of `refresh_after_days` back applies, and `src/state.rs` holds no run-start field today (only per-document `applied_at`) — .engineering/planning/story/connectors-source-walks.md:36
story:file-records — the revision narrowed re-delivery to "editing one section changes only that record's content hash", which does not observe the Work's "an edited record is delivered again", and `SeenState::select` re-delivers changed text only past the refresh window — .engineering/planning/story/file-records.md:36

What I read: 10 artifacts. These were the 7 stories, `story:spec-standalone-types` including its "Organisation-scale types" section, the epic and `review-result:org-acceptance-round-1`, all in full via `aep plan artifact show`. I also checked `src/run.rs`, `src/main.rs`, `src/state.rs` and `tests/e2e.rs` with `git grep`.

Round-1 findings:

| Round-1 finding | Status |
|---|---|
| `spec-organisation-types` | fixed, now one equality check in `story:spec-standalone-types` |
| `run-gate` (`facts_refused`) | fixed: `facts_refused` exists at `src/run.rs:36,200,250`, and `tests/e2e.rs:221` already makes a stand-in produce a refused fact |
| `quality-judge` (verdict location) | fixed: output paths and format are named |
| `connectors-source-walks` | split into two checks, and the split left the gap above |
| `file-records` | split into two checks, and the split left the gap above |
| `redaction-names-and-gate` | split into two checks, each observable, no finding |

`structured-from-files-and-drops` and `document-time-as-valid-time` were observable before and remain so.

What I could not establish:
- Whether `ekr sample` and `ekr fact-quality` exist. `ekr` is not installed here, so this does not set the verdict.
- Whether `ekr.fact-quality/1` counts "unclear" verdicts in the 0.9 pass rate. Not set in the story.

Out of my lane, not counted in the verdict:
- `story:spec-standalone-types` Work (optional cost, `--codex` plumbing, a Codex backend failure message) has no check in its single "behaves as before" acceptance (scope).
- `story:run-gate` names `ekr quality` measures without listing them (scope or design).

```findings
[
  {"file": ".engineering/planning/story/connectors-source-walks.md", "line": 36, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the window check feeds {since} from a state fixture, so nothing observes that a run records its start instant or that the first-run default of refresh_after_days back applies, and src/state.rs holds no run-start field today (only per-document applied_at)"},
  {"file": ".engineering/planning/story/file-records.md", "line": 36, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the revision narrowed re-delivery to 'editing one section changes only that record's content hash', which does not observe the Work's 'an edited record is delivered again', and SeenState::select re-delivers changed text only past the refresh window"}
]
```

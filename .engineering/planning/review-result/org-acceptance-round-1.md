---
format: aep.planning-md/3
id: review-result:org-acceptance-round-1
kind: review-result
status: archived
title: Acceptance critic, organisation scale, round 1
relations:
- reviews: epic:organisation-scale-instance
- reviews: story:spec-organisation-types
- reviews: story:connectors-source-walks
- reviews: story:file-records
- reviews: story:redaction-names-and-gate
- reviews: story:structured-from-files-and-drops
- reviews: story:document-time-as-valid-time
- reviews: story:run-gate
- reviews: story:quality-judge
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:38Z", actor: "human:timo", revision: 2}
---
needs-revision
story:spec-organisation-types — the acceptance joins two independent outcomes ("`tests/spec_compat.rs` still passes unchanged, and a spec setting every new field to its default … equal the run without those fields"), and the second has no named test, because `tests/spec_compat.rs` is declared unchanged yet is the only candidate and does not yet set the new fields — .engineering/planning/story/spec-organisation-types.md:200
story:connectors-source-walks — one sentence joins two independent outcomes (3 pages with 2 children each yielding 6 documents, and "a second run whose `{since}` equals the first run's start instant"), so paging can pass while the since-window fails — .engineering/planning/story/connectors-source-walks.md:39
story:file-records — the acceptance joins three independent outcomes (7 documents from filter and sections, the `[context]` on the second thread record, re-delivery of exactly one edited section), so any can pass while the others fail — .engineering/planning/story/file-records.md:39
story:redaction-names-and-gate — the acceptance joins two independent outcomes (masking sends the model none of the planted values, and a separate batch is refused with the store head unchanged), so masking can pass while the refusing gate fails — .engineering/planning/story/redaction-names-and-gate.md:38
story:run-gate — the acceptance rests on the measure `facts_refused`, which the run report does not carry today (only `documents_applied` appears in `SourceRan`) and which the Work does not add, so "naming `facts_refused = 1`" cannot be observed — .engineering/planning/story/run-gate.md:38
story:quality-judge — "writes the verdicts beside it" names no location or format for the verdicts, so a second reader cannot get the same answer on whether they were written — .engineering/planning/story/quality-judge.md:35

What I read: 9 artifacts (the epic and all 8 stories) in full via `aep plan artifact show`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I also ran `git grep` for `facts_refused`, `documents_applied` and `fact-quality`, and listed `tests/`.

What I could not establish:
- Whether `ekr sample`, `ekr fact-quality` and `ekr.fact-quality/1` exist. `ekr` is not installed here, and this does not set the verdict.
- Whether `facts_refused` arrives from the EKR partial-apply upstream blocker. The story does not say so.
- The acceptances of `structured-from-files-and-drops` and `document-time-as-valid-time` are observable and show a before/after transition, so I raised no finding on them.

Out of my lane, and not counted in the verdict:
- `story:spec-organisation-types` lists `tests/spec_compat.rs` in its scope while the acceptance says it stays unchanged (design or parallel-safety).
- Several stories depend on unpromoted `upstream-blocker:*` records (scope or design).

```findings
[
  {"file": ".engineering/planning/story/spec-organisation-types.md", "line": 200, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins two independent outcomes (spec_compat.rs still passes unchanged, and a spec setting every new field to its default equals the run without those fields) and the second has no named test, because spec_compat.rs is declared unchanged yet is the only candidate and does not yet set the new fields"},
  {"file": ".engineering/planning/story/connectors-source-walks.md", "line": 39, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "one sentence joins two independent outcomes (3 pages with 2 children each yielding 6 documents, and a second run whose {since} equals the first run's start instant), so paging can pass while the since-window fails"},
  {"file": ".engineering/planning/story/file-records.md", "line": 39, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins three independent outcomes (7 documents from filter and sections, the [context] on the second thread record, re-delivery of exactly one edited section), so any can pass while the others fail"},
  {"file": ".engineering/planning/story/redaction-names-and-gate.md", "line": 38, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins two independent outcomes (masking sends the model none of the planted values, and a separate batch is refused with the store head unchanged), so masking can pass while the refusing gate fails"},
  {"file": ".engineering/planning/story/run-gate.md", "line": 38, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance rests on the measure facts_refused, which the run report does not carry today (only documents_applied appears in SourceRan) and which the Work does not add, so 'naming facts_refused = 1' cannot be observed"},
  {"file": ".engineering/planning/story/quality-judge.md", "line": 35, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "'writes the verdicts beside it' names no location or format for the verdicts, so a second reader cannot get the same answer on whether they were written"}
]
```

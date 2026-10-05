---
format: aep.planning-md/3
id: review-result:standalone-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, standalone 1.0, round 2
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
story:codex-model-backend — the acceptance requires the run report to show `"cost_usd": null`, but the report's `cost_usd` is a required `Decimal` in the spec and a `"0.0000"`-style string in `src/ports.rs`, and no story's Work makes it nullable (spec-standalone-types makes only the model answer's cost optional, and structured-source says the report shows "cost 0"), so the acceptance cannot be met as written — .engineering/planning/story/codex-model-backend.md:38

```findings
[
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 38, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance requires the run report to show `\"cost_usd\": null`, but the report's `cost_usd` is a required `Decimal` in the spec and a `\"0.0000\"`-style string in `src/ports.rs`, and no story's Work makes it nullable (spec-standalone-types makes only the model answer's cost optional, and structured-source says the report shows \"cost 0\"), so the acceptance cannot be met as written"}
]
```

What I read:
- All 10 stories in full, plus `review-result:standalone-acceptance-round-1`, using `aep plan artifact show`, `kinds` and `lifecycle story`.
- In the tree I ran `git grep` for `cost_usd`, `documents_applied`, `RestoreSnapshot` and `AdoptInstance`, and read `spec/domains/instance.yaml`, `src/ports.rs`, `src/main.rs` and `.github/workflows/check.yml`.

Round-1 findings:

| Story | Round-1 finding | Now |
|---|---|---|
| `spec-standalone-types` | three joined outcomes | fixed, the acceptance is one statement and the refusals are gone |
| `store-backend-per-instance` | skipped test passes | fixed, the test must run in the `Check` workflow, with the run URL as evidence |
| `run-snapshots` | joined outcomes and unchecked refusals | fixed, the refusals move to the `RestoreSnapshot` conformance scenarios |
| `adopt-existing-store` | no observable | fixed, head N is unchanged after adopt and N+1 after a run, with `documents_applied: 2` |
| `docs-for-1-0` | no page named, three joined checks | fixed, the evidence is a PR table with a test or command output per statement |
| `release-1-0` | no expected statuses or record place | fixed, exit 0, `documents_applied` above 0 and HTTP 200, each recorded as a `test_result` on the story |
| `codex-model-backend` | no interface named, commit-message clause | the interface and the clause are fixed, but the `null` finding above is new |

I approve `redaction-before-model`, `structured-source` and `release-pipeline` as before.

What I could not establish:
- I did not run `task check`.
- I could not run `aep plan evidence`, because the verb does not exist here, so I did not confirm how a `test_result` is recorded on `release-1-0`. The `test_result` kind itself appears in other stories' transition records.
- `check.yml` has no Docker service entry, so whether the PostgreSQL container needs one is outside my lane.
- Out of my lane: `story:timer-runs-unattended` and `story:seen-documents-modelled` are outside the set I was given, and so are the `depends_on` edges.

---
format: aep.planning-md/3
id: story:quality-judge
kind: story
status: implemented
title: An operator can measure how well facts are supported by their evidence
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
- depends_on: story:adopt-existing-store
- depends_on: story:run-gate
- depends_on: story:document-time-as-valid-time
- depends_on: story:extraction-supersedes
scope:
- confidence: cited
  path: src/extract.rs
- confidence: cited
  path: src/lib.rs
- confidence: cited
  path: src/main.rs
- confidence: inferred
  path: src/quality.rs
- confidence: inferred
  path: tests/quality.rs
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T09:15:47Z", actor: "agent:claude", revision: 13, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-06T09:15:47Z", actor: "agent:claude", revision: 14, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-06T10:15:05Z", actor: "agent:claude", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}}
---
## Outcome

An operator can ask how often an instance's facts are supported by the evidence they cite.

## Work

- `cortex quality <instance> --sample N`: `ekr sample` draws N facts with their evidence bytes;
  each batch of 20 goes to the instance's model with no tools, asking whether the evidence supports
  the fact (yes / no / unclear with a reason); `ekr fact-quality` turns the verdicts into a pass rate
  with its interval. Cost is reported like a run's.
- The verdict call uses the instance's model backend with its own system prompt: `src/extract.rs`
  gains `Model::ask_with(system_prompt, ...)`, which `Model::ask` calls with the extraction prompt
  (`src/extract.rs:20,182`).
- Output under the instance directory: `quality/<UTC stamp>/verdicts.jsonl` (one
  `cortex.quality-verdict/1` object per fact: fact id, verdict, reason) and
  `quality/<UTC stamp>/fact-quality.json` (the `ekr.fact-quality/1` document).

## Acceptance

`tests/quality.rs`: with a stand-in model answering 18 yes and 2 no for a sample of 20,
`quality/<stamp>/fact-quality.json` reports a pass rate of 0.9, and `verdicts.jsonl` has 20 lines,
2 of them `no`.

## Depends on

`story:spec-standalone-types` (the optional-cost seam), `story:adopt-existing-store` and
`story:run-gate` (both edit `src/main.rs` and `src/lib.rs` before it), `story:codex-model-backend`
and `story:document-time-as-valid-time` (both edit `src/extract.rs`).
`story:web-pages-cited-as-urls` comes after this story (it edits `src/main.rs` and
`src/extract.rs`; its edge records it).

## Scope

Landed 2026-10-06 in `fdfdbbd` (wave 20261006g), with a test fix `1bf9fd3`.

- **Files:** `src/quality.rs` (new), `src/extract.rs` (`Model::ask_with`, `Refused` with cost), `src/main.rs`, `src/lib.rs`, `src/ports.rs`, `spec/domains/instance.yaml` and `spec/components.yaml` (`MeasureQuality`, `QualityMeasured`, `QualityVerdict`), `generated/`, `tests/quality.rs` (new), `tests/conformance.rs`, `website/docs/commands.md`, `AGENTS.md` (scenario count)
- **Review:** one adversary pass, 4 findings fixed (rare names reached the judge, a failed call's cost, unclear untested, the home lock held through model calls)
- **Open:** a failed extraction call's cost never reaches a run's `cost_usd` (`src/run.rs`), pre-existing
- **Closed 2026-10-07:** `rate` and `cost_usd` are on `QualityMeasured` since wave 20261007b (`story:events-carry-measurements`), after ESS 0.55.0 settled https://github.com/beyond10x/ess/issues/467

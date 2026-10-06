---
format: aep.planning-md/3
id: story:quality-judge
kind: story
status: draft
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
revision: 12
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

`src/quality.rs` (new), `src/extract.rs` (`Model::ask_with`), `src/lib.rs`, `src/main.rs`,
`tests/quality.rs` (new).

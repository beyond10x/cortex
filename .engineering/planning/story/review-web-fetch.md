---
format: aep.planning-md/3
id: story:review-web-fetch
kind: story
status: active
title: A Web run reports per-URL failures and stores no text outside its evidence
summary: Findings 7 and 10 of the 0.7.2 review
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 3}
---
## Outcome

A Web source's run report carries every per-URL failure and the provider's completeness, and a
stored Web fact never holds text outside the evidence it cites.

## Work

Findings 7 and 10 of the external review of cortex 0.7.2 (2026-11-23):

- 7: when Connectors returns the Web contract's per-URL failure for the only requested URL,
  fetch-only shows zero fetched and the run returns `ran` with no stop reason; the provider's
  incomplete-result flag is ignored (`src/sources.rs`, `src/run.rs`). Carry per-URL failures and
  completeness into the run report.
- 10: a page description longer than the 16,384-byte evidence payload goes into the prompt and is
  written as a WebPage property, while its tail is outside the evidence (`src/evidence.rs`,
  `src/extract.rs`). Apply the byte budget to metadata, model input and stored text alike.

## Acceptance

A test per finding reproduces the reviewed scenario, fails on 0.7.2's code and passes on this
change; `website/docs/operating.md` describes the run report fields.

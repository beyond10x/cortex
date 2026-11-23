---
format: aep.planning-md/3
id: story:review-schema-conformance
kind: story
status: active
title: cortex accepts what its schema accepts and conformance uses the real loader
summary: Findings 25 and 26 of the 0.7.2 review
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:54:01Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:54:01Z", actor: "human:timo", revision: 3}
---
## Outcome

cortex accepts every instance document its published schema accepts, and the conformance suite
runs scenario documents through the real loader.

## Work

Findings 25 and 26 of the external review of cortex 0.7.2 (2026-11-23):

- 25: `timeout_s: 1.0` is valid under the published draft 2020-12 schema but `create` refuses it
  as not a whole number (`src/model_map.rs`). Accept mathematically integral numbers.
- 26: all 86 create, adopt and update steps of `tests/conformance.rs` pass a fixed placeholder
  spec to the generated behaviour; their supplied documents name a format other than
  `cortex.instance/1`, and one scenario labelled `created` holds a document the CLI refuses
  (`tests/conformance.rs`, `spec/suite.json`, `src/spec.rs`). Run representative scenario
  documents through the real loader and assert the command outcome.

## Acceptance

`create` accepts `1` and `1.0` and refuses `1.5`; the conformance suite loads each create, adopt
and update document through `src/spec.rs`, and a scenario whose document the loader refuses fails.

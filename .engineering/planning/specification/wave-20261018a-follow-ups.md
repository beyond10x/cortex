---
format: aep.planning-md/3
id: specification:wave-20261018a-follow-ups
kind: specification
status: approved
title: 'Wave 20261018a: follow-ups of wave 20261017d'
relations:
- specifies: story:structured-lookup-across-types
- specifies: story:ontology-names-masked
- specifies: story:fetch-stop-point
- specifies: story:partial-run-counts
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 3}
---
## Wave 20261018a: follow-ups of wave 20261017d

Opened 2026-10-18, `aep:implementing` wave mode. Based on `origin/main` `43715c17` (release
0.6.1). Integration branch `wave/20261018a`, tree `cortex-w18a-int`. Release 0.6.2.

## Units

| unit | story | tree |
|---|---|---|
| lookup | `story:structured-lookup-across-types` | `cortex-w18a-lookup` |
| onto | `story:ontology-names-masked` | `cortex-w18a-onto` |
| fetch | `story:fetch-stop-point` | `cortex-w18a-fetch` |
| counts | `story:partial-run-counts` | `cortex-w18a-counts` |

Width: 4, every drafted follow-up. Code scopes are disjoint: lookup is `src/structured.rs` and
`RecordMapping`; onto is `Known::from_ontology` in `src/extract.rs`; fetch is the fetch child in
`src/sources.rs` and `src/connectors.rs` and the retry wait in `extract::Model::ask`; counts is the
status write in `src/main.rs` and `src/status.rs`. All four edit `RunSource`'s paragraphs in
`spec/domains/instance.yaml`, so each works in its own tree, and `generated/` is regenerated on the
integration branch after each merge.

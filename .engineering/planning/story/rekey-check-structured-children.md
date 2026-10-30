---
format: aep.planning-md/3
id: story:rekey-check-structured-children
kind: story
status: draft
title: cortex update refuses a change of a structured child's operation or mapping.id with seen records
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

`cortex update` refuses a spec that changes a structured connectors source's `children[].operation`
or a child's `mapping.id` while the source has seen documents, because both are part of a child
record's key. Today `keyed_by` (`src/main.rs`) compares the parent's `adapter`, `operation` and
`mapping.id` only.

## Acceptance

- On an instance whose structured connectors source has seen child records, `cortex update` with
  a changed `children[].operation` exits 1 `rekey-refused` naming the source and the field, and the
  frozen spec and state are unchanged (new test).
- The same for a changed child `mapping.id` (new test).
- The same changes on a source with empty seen state are accepted.

## Source

- `review-result:adversary-w30a-outcome-pass-1` (mapping.id finding, `src/main.rs:1488`) and the
  correction 420b8144 that added the parent's `mapping.id` only.
- Wave 20261030a coordinator review of the unit reports.

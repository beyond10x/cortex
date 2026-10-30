---
format: aep.planning-md/3
id: story:ask-serve-error-names-path-once
kind: story
status: draft
title: A cortex ask serve error names the spec path once
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

An error from `cortex ask serve` names the spec path once. Today it prints the path twice
(`asker.yaml: asker.yaml: …`). Pre-existing.

## Acceptance

- `cortex ask serve` on a spec that fails to load prints an error whose text holds the spec path
  exactly once (new test asserting the count of the path in stderr).
- The other spec-loading commands keep their current single-path messages (existing tests green).

## Source

- Wave 20261030a coordinator review of the unit reports.

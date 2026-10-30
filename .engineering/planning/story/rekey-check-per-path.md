---
format: aep.planning-md/3
id: story:rekey-check-per-path
kind: story
status: draft
title: The re-key check compares a structured files source's paths per entry
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

`cortex update` refuses a structured files source's change as a re-key only when the change
re-keys documents already seen, not whenever its `paths` list differs.

## Why

Found during wave 20261030a (unit shapes review): the re-key check compares a structured files
source's `paths` as one whole list, so adding a path, or reordering the list, is refused as a
re-key although no seen document's key changes. Pre-existing before the wave.

## Work

1. Compare `paths` per entry: a removed or changed entry can re-key, an added one cannot.
2. Tests for an added path (updated) and a changed path (rekey-refused).

## Acceptance

- Adding a path to a structured files source with seen documents answers `updated`.
- Changing an existing path answers `rekey-refused`.

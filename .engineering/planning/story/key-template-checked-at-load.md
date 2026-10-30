---
format: aep.planning-md/3
id: story:key-template-checked-at-load
kind: story
status: draft
title: A malformed connectors key template is refused when the spec loads
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A connectors source's `key` template is checked for syntax when the spec loads (`src/spec.rs`), so
a malformed template is refused by `create`, `update` and `adopt` instead of skipping every record
at run time, as it does today.

## Acceptance

- A spec whose connectors `key` template is malformed (for example an unclosed `{`) is refused at
  `cortex create` and `cortex update` with a message naming the source and the template (new test).
- A well-formed template that names a field a record lacks still loads, and that record goes to
  `skipped` at run time, as today.

## Source

- Wave 20261030a coordinator review of the unit reports.
- `story:record-key-template`: the key renders with `structured::render_template`
  (`src/structured.rs:618`), which answers `None` for a missing field.

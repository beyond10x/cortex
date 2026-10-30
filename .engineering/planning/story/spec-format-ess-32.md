---
format: aep.planning-md/3
id: story:spec-format-ess-32
kind: story
status: draft
title: The specification is written in format ess/32
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

`spec/system.yaml` declares `format: ess/32` instead of `ess/22`, and any change the newer format's
rules make to the generated suite is reviewed. The precedence rules of ess/27 and ess/29 can change
which scenarios the suite holds.

## Acceptance

- `spec/system.yaml:1` reads `format: ess/32` and `ess specify validate` exits 0.
- `task generate` then `task drift` is clean.
- The diff of `spec/suite.json` against the ess/22 suite is listed in the change, with each added,
  removed or changed scenario named, and `tests/conformance.rs` passes with its scenario floor set
  to the new count.

## Source

- Wave 20261030a coordinator review of the unit reports (hardening, "follow-up story, not this wave").

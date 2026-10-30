---
format: aep.planning-md/3
id: story:redaction-check-command
kind: story
status: draft
title: A consumer checks redaction through a cortex redact command
tags:
- downstream-request
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A consumer can check what cortex's redaction stores and shows a model for a given spec and text
without linking `cortex-cli` as a library, so its own builds keep their `serde_json` features.

## Why

Reported by a downstream instance on 2026-10-30 against v0.6.1, and read in v0.7.0's
`Cargo.toml:28` (`serde_json` with `features = ["arbitrary_precision"]`) and
`generated/spec-types/Cargo.toml:15-16` (`default = ["exact-numbers"]`). A consumer that adds
`cortex-cli` as a dev-dependency to run `redact::compile` over its own spec gets
`arbitrary_precision` unified into its test builds; a TOML config read through `serde_json::Value`
into a struct with an `i64` field then fails with `invalid type: map, expected i64`.

## Work

1. A `cortex redact --spec <file>` subcommand: reads text on stdin and prints, as JSON, what a run
   would store (mask, then the policy's scrub), what the model would see (one batch's
   replacement) and the classes `refuse_if_left` would find left.
2. Its output format is generated from the specification like the other command outputs.

## Acceptance

- `cortex redact --spec <file>` over a credential line prints the masked stored text, the replaced
  shown text and an empty `left`, and exits 0.
- With `refuse_if_left: [Credential]` and a credential the policy misses, `left` names
  `Credential` and the command exits non-zero.
- A consumer test calling the subcommand needs no `cortex-cli` dependency.

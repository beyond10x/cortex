---
format: aep.planning-md/3
id: story:update-removed-instance-precedence
kind: story
status: draft
title: cortex update on a removed instance answers not-active before a seed refusal
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

`cortex update` on a removed instance answers the same outcome whatever the new spec changes,
and a scenario pins it.

## Why

ESS hardening pass 1 (`review-result:hardening-w30a-pass-1`, guard analysis): a removed
instance updated with a changed seed answers "seed change refused" rather than "not active"
(`spec/domains/instance.yaml:1698`), and no scenario pins either. The ess/27 precedence order
(request, existence, held state, related rows, accepting) puts the held-state refusal first.

## Work

1. Order UpdateInstance's branches so the held-state refusal answers before the seed-change
   refusal; change the implementation to match.
2. An authored scenario for a removed instance updated with a changed seed.
3. Acknowledge the outcome change in `spec/compat-ack.json` with its reason.

## Acceptance

- The scenario passes and fails with the two branches swapped back.
- `task compat` exits 0 with the acknowledgement and non-zero without it.

---
format: aep.planning-md/3
id: story:name-redaction-boundaries
kind: story
status: draft
title: A name beside marks, digits or digraphs is replaced or its limit documented
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A known or rare name that sits beside CJK text, an underscore, a footnote digit or inside a
lower-case-prefixed file name, or starts with a titlecase digraph, is replaced before a model sees
it, or the limit is decided and documented as intended.

## Why

Four adversary cases from the 0.6.1 redaction work (`tests/redaction_beside_marks_adv.rs`, commit
7d9a0948, 2026-10-17) were kept `#[ignore]`d without a story:

- a known name glued to CJK text or an underscore is shown (`:149`);
- a name with a footnote digit is shown (`:164`);
- a name in a lower-case-prefixed file name is shown; the spec admits identifiers through
  underscores, so this one is a judgement (`:180`);
- a name starting with a titlecase digraph is shown (`:198`).

## Acceptance

- Each case runs without `#[ignore]` and passes, or names this story in its reason with a line
  under the CHANGELOG's known limits that says why it stays.

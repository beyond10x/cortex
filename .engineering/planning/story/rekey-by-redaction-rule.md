---
format: aep.planning-md/3
id: story:rekey-by-redaction-rule
kind: story
status: draft
title: A redaction rule that rewrites structured ids is refused or its re-extraction reported by cortex update
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A `cortex update` that adds or changes a `redaction.rules` entry matching a structured source's
ids no longer re-keys every seen record silently. Today a structured record's seen key comes from
`Source::identity_in` (`src/structured.rs:852`), which digests the id whenever the redactor would
change it, and `keyed_by` (`src/main.rs:1509`) looks only at source fields, so the update is
accepted and the next run re-extracts every record under new keys.

## Acceptance

- The story records one decision before implementation: refuse the update as `rekey-refused`, or
  accept it and report the re-extraction it causes.
- If refuse: on an instance with seen structured records, an update whose new rule matches their
  ids exits 1 `rekey-refused` naming the source and `redaction.rules`, and state is unchanged
  (`a_redaction_rule_that_rewrites_structured_ids_with_seen_records_is_refused`,
  `tests/rekey_adversary_pass2.rs:147`, passes without `#[ignore]`).
- If report: the update's detail names each source whose seen keys change and the count, and the
  next run's `documents_new` equals that count (new test).
- A rule that matches no seen id is accepted either way.

## Source

- `review-result:adversary-w30a-outcome-pass-2`, finding 1 (`src/main.rs:1509`,
  `src/structured.rs:852`).
- Wave 20261030a coordinator review of the unit reports.

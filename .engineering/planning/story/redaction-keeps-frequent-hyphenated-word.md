---
format: aep.planning-md/3
id: story:redaction-keeps-frequent-hyphenated-word
kind: story
status: draft
title: A frequent hyphenated word is kept whole, and the left-over check agrees with what was replaced
refs:
- provider: github
  reference: beyond10x/cortex#71
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A capitalised word that a batch holds more than `rare_limit` times is kept whole, hyphenated or
not: no part of it is replaced as a rare name, and the `refuse_if_left` check finds nothing left
of it. A batch of commit messages that each carry `Co-Authored-By:` is applied, not refused.

## Why

Reported against 0.5.1 (issue #71): with `RareName` in `classes` and `refuse_if_left`, a
frequent `Co-Authored-By` trailer had its tail replaced (`Co-[Name-1]`), and the head left
standing (`Co`) was then found as a rare name, so the whole batch was refused: one batch in three
on a commit-message source.

Verified in the code:

- `Batch::reserve` counts `Co-Authored-By` as one word (`capitalised_word`, which keeps
  `Jean-Luc` whole).
- `Redactor::scan` rejects a non-rare candidate and retries at the next character
  (`retry = next_char(text, whole.start)`); the pattern then matches `Authored-By` inside the
  word, which was never counted, so it is rare and replaced.
- `Batch::left` scans what the model would be shown and finds `Co`, never counted, as rare.

## Domain relations

No new noun. The `RareName` rule in `cortex.instance.RedactionPolicy`'s comment (and the
`RedactionClass` it names) states that a word is taken whole or not at all.

## Work

- Spec first: state in the `RareName` comment of `spec/domains/instance.yaml` that a capitalised
  word, hyphenated or not, is replaced whole or not at all, and that no part of a word seen more
  than `rare_limit` times is a rare name; validate, `task generate`, `task drift`.
- `src/redact.rs`: a rare-name candidate rejected as not rare is skipped to its end, not retried
  inside it; the left-over check (`Batch::left`) does the same, so it agrees with what was
  replaced.
- Tests first: a unit test in `src/redact.rs` and a gate test in `tests/redaction_gate.rs` with
  the synthetic reproduction of issue #71.
- CHANGELOG Fixed entry naming issue #71.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Three documents in one batch, each ending `Co-Authored-By: Example Person <person@example.org>`,
  with `classes: [RareName]`, `rare_limit: 1`, `refuse_if_left: [RareName]`: the batch is applied,
  and the text the model is shown holds `Co-Authored-By` as written.
- A hyphenated capitalised word seen once (`Jean-Luc`) is still replaced whole by one placeholder.
- A rare capitalised word next to a frequent hyphenated one is still replaced, and a rare name
  left in the shown text still refuses the batch (existing gate tests unchanged).

## Not in this story

- Any change to how often a word must be seen, or to stop words.

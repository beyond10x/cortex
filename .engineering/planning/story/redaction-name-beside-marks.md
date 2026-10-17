---
format: aep.planning-md/3
id: story:redaction-name-beside-marks
kind: story
status: implemented
title: A rare name beside an underscore, CJK text or a known name is replaced
refs:
- provider: github
  reference: beyond10x/cortex#74
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T22:06:51Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-16T22:06:51Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T02:21:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A rare capitalised name reaches the model as a placeholder wherever it stands in a word. That
covers a name in underscore italics (`_Rowan_`), a Latin name written straight after CJK text
(`会议上Rowan发言了`), and the rare head of a word whose tail is a known name
(`Schmidt-Müller` with `Müller` known). The `refuse_if_left` check finds such a name when it is
left in place.

## Why

Found by the adversary on issue #71's fix (tests/redaction_hyphen_adv.rs, cases
`a_name_in_underscore_italics_is_not_shown`, `a_latin_name_glued_to_cjk_text_is_not_shown`,
`the_rare_half_of_a_name_hyphenated_to_a_known_name_is_not_shown`, kept ignored). All three come
from before that fix:

- `src/redact.rs` `capitalised` rejects a word that has `_` or any alphanumeric letter, CJK
  included, before it, so the name is neither a rare-name candidate nor seen by the gate.
- When a rare-name match overlaps a known name, the scan retries at the next character, and the
  head before the known name is never counted or replaced.

## Domain relations

No new noun. The `RareName` rule in the comment of `cortex.instance.RedactionPolicy` states
which characters bound a word.

## Work

- Spec first: state in the `RareName` comment of `spec/domains/instance.yaml` that `_` and a
  letter of a script without case bound a word. Then validate, `task generate` and `task drift`.
- `src/redact.rs`: change the word boundary in `capitalised`, and in the known-name overlap path
  count and replace the rare head.
- Tests: un-ignore the three adversary cases.

## Acceptance

- `ess specify validate` passes, and `task drift` is clean after `task generate`.
- The three adversary cases pass with no part of the name shown. With `refuse_if_left:
  [RareName]` and the replacement off, each case is refused.
- `snake_case_Identifier` and other words glued to a lower-case Latin letter are not rare names
  (existing tests unchanged).

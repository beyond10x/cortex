---
format: aep.planning-md/3
id: review-result:adversary-w30b-unit-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit-takeover fix
relations:
- reviews: story:unit-takeover-across-homes
revision: 1
---
unit: unit (wave 20261030b), unit/w30b-unit at 447fe978, adversary pass 1 (opus), cases in tests/w30b_unit_adversary.rs (cc9f34b2)
verdict: NEEDS-CHANGE
needs-coordinator: none

1. `adv_w30b_the_holder_is_named_when_the_other_home_also_has_an_instance_of_the_name`: red. The
   other home holds `a-b/x` and `a/b-c`; `create a-b/c` in a second home is refused, but the
   refusal names no holder and says to remove the instance of the same name, which does not hold
   the unit.
2. `adv_w30b_a_source_unit_colliding_with_another_homes_live_viewer_is_refused`: green.
3. `adv_w30b_a_homes_own_update_is_not_refused_by_a_colliding_instance_elsewhere`: green.

Held: every unit write path goes through `foreign`; no partial state on refusal; names are
`[a-z0-9-]` only; an unreadable holder home is refused; a removed holder is taken over.

```findings
- file: src/schedule.rs
  line: 446
  category: spec-gap
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: when the other home also has an active instance of the requesting name, foreign returns before asking unit_holder, so the refusal names no holder and points at an instance that does not hold the unit
```

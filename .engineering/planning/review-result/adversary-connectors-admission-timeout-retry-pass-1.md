---
format: aep.planning-md/3
id: review-result:adversary-connectors-admission-timeout-retry-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: connectors-admission-timeout-retry (wave 20261008a)'
relations:
- reviews: story:connectors-admission-timeout-retry
revision: 1
---
## Adversary pass 1: connectors-admission-timeout-retry (wave 20261008a)

Against `717842a` (base `413de0e`). Verdict: nothing found. Five cases added in `tests/connectors_admission_retry_adv.rs`, all green against the unit; package suite 535 passed, 0 failed.

| case | asserts |
|---|---|
| `adv_a_timeout_at_the_provider_stage_is_not_retried` | `timeout` at `provider` answers `fetch-failed` after 1 invocation |
| `adv_lapsed_then_timeout_then_answer_applies_in_three_invocations` | exit 0, 3 invocations, 1 forced revalidate |
| `adv_timeout_then_lapsed_then_answer_applies_in_three_invocations` | exit 0, 3 invocations, 1 revalidate |
| `adv_lapsed_timeout_lapsed_fails_after_three_invocations` | exit 1 after 3 invocations |
| `adv_timeout_timeout_timeout_fails_after_two_with_the_base_reason` | `fetch-failed` after 2 invocations, reason unchanged |

Notes, not findings: the timeout is matched on the message suffix `: timeout at admission` (`src/connectors.rs`), so an `operations describe` timing out at admission is retried too (a read; bounded). The doc comment of `Connectors::invoke` said "once"; fixed in `8e2bcee`.

```findings
[]
```

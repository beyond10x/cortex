---
format: aep.planning-md/3
id: story:one-store-read-per-run
kind: story
status: implemented
title: A run reads the store once, not twice per batch
summary: The `refuse_if_left` pre-check and the per-batch known-entity lookup share one snapshot index built once per run and updated after each apply
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated/cortex-model
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: cited
  path: src/known.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: tests/known_entities.rs
- confidence: inferred
  path: tests/known_entities_adversary.rs
- confidence: cited
  path: tests/redaction_gate.rs
- confidence: cited
  path: website/docs/operating.md
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 13}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 14}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:11Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A run builds the known-entity index from one `ekr snapshot` and keeps it for the whole run,
updating it from each applied batch's entities. Today the `refuse_if_left` pre-check calls
`known::for_batch` once per batch (`src/run.rs:919-929`), and the batch loop calls it again
before every model call (`src/run.rs:964-967`); each call runs `store.snapshot()`
(`src/known.rs:155-169`). The downstream deployment (PostgreSQL, about 108k assertions) measured
3.5 to 4 min per snapshot: 34 min to the first batch, then about 6.5 min per batch (cb3 report
2026-10-30, C-01; Opus performance review P-01, P-02).

## Acceptance

1. With a stand-in `ekr` that logs its argv, a three-batch run under a `refuse_if_left` policy
   issues at most one `snapshot` (new test).
2. A name applied in batch k is offered as known in batch k+1's prompt (new test).
3. The pre-check's pseudonymised batch 0 is reused by the loop, so batch 0 is redacted once
   (P-14 half).
4. `tests/known_entities.rs` and `tests/redaction_gate.rs` stay green.

## Findings it closes

C-01, P-01, P-02, P-14 (double redaction of batch 0).

## Scope

Derived 2026-10-30 by `story-scoper` against `b61a56c8` (src/, tests/, spec/ and generated/ unchanged from main `a870a628`). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/run.rs` `process_in` (`src/run.rs:567-1212`), the batch path of a model-extracting run. Cited.
- **Files:** `src/run.rs:919-929`, the `refuse_if_left` pre-check calling `known::for_batch` once per batch. Cited, confirmed (the `if` block closes at 930).
- **Files:** `src/run.rs:964-967`, the per-batch `store.ontology()` and `known::for_batch` in the queue loop. Cited, confirmed. A batch that is split and re-queued (`src/run.rs:1008-1014`) runs that lookup again for each half, so today there are more snapshots than batches. Cited.
- **Files:** `src/known.rs:155-169`, `for_batch` calling `store.snapshot()` at 167. Cited, corrected to 155-170: the doc comment is 155-156, the fn is 157-170.
- **Files:** `src/run.rs:1160-1166`, where the run's names grow from `extract::entity_names(&restored)` after `store.apply` at 1131. Cited (read). It is the natural point to update the index after each apply. Inferred.
- **Files:** `src/run.rs:968-1001`, the `Some(r)` arm that calls `pseudonymise` and `refused` again. Batch 0's `(redact::Batch, shown, known)` from the pre-check would be reused here, only when the first item popped is unsplit (`depth == 0`, `n == 0`) (acceptance 3, P-14). Cited lines; the change is inferred.
- **Symbols:** `known::for_batch` (signature changes from `&crate::ekr::Store` to a run-held index, or a new sibling fn). Cited.
- **Symbols:** `known::store_hits` (`src/known.rs:44-101`, the per-call linear scan of `graph.graph.nodes`), likely reused or replaced by an index build. Inferred.
- **Symbols:** `pseudonymise` (`src/run.rs:2274`), `refused` (`src/run.rs:2106`) and `redact::Redactor::batch` (`src/redact.rs:1113`) are called by the changed code but not changed themselves. Inferred.
- **Symbols:** `crate::ekr::Store::snapshot` (`src/ekr.rs:496-499`) is unchanged. Cited.
- **Also likely:** a new index type in `src/known.rs` (for example a folded-term → `KnownEntity` map built from one snapshot and the ontology). Inferred.
- **Also likely:** `src/known.rs:1-5` module doc, which says "the store is read once per batch (`ekr snapshot`)" and must be reworded. Cited (text read); the need to edit it is inferred.
- **Also likely:** hoisting `store.ontology()` at `src/run.rs:964` out of the loop. The story does not ask for it, and the ontology is also read at 901 and 920. Inferred.
- **Spec:** `spec/domains/instance.yaml:1702-1717` (`UNMAPPED:` comment: "before each batch is sent to the model, the run reads the store once (`ekr snapshot`)") must say "once per run, updated after each apply". Cited (text). `instance.yaml:266-269` ("a run computes the list per batch") stays true. Cited.
- **Spec:** no type change. `cortex.instance.KnownEntity` (`instance.yaml:270-276`) and `KnownOrigin` keep their fields. Cited.
- **Generated:** no type or doc text changes; `generated/cortex-model/src/instance.rs:355` carries only the type name, and a search found no copy of the comment text under `generated/`. Cited. Any edit to `instance.yaml` may move the `source_digest` (`generated/cortex-model/plan.json:5`, `generated/cortex-model/src/lib.rs:2`) and so rewrite the digest header of every `generated/**` file and `spec/suite.json`, as spec commit `1f95f927` did. Inferred: not established whether a YAML comment counts toward the digest.
- **Documents:** `website/docs/operating.md:202` ("Before each batch is sent, the run reads the store once") and `:225-227` ("`ekr snapshot` reads the whole graph … once per batch, so on a large store each batch takes that read's time") are now wrong and need rewording. Cited.
- **Tests (new):** acceptance 1 needs a stand-in `ekr` that logs its argv, a three-batch run and a `refuse_if_left` policy. The argv-logging pattern to copy is `tests/adversary_20261016a.rs:25-28` (`printf … "$*" >> "$R/ekr-argv.log"` around the real `ekr`). Cited (pattern read). The new tests' home, `tests/known_entities.rs` or a new `tests/one_store_read.rs`, is inferred.
- **Tests (new):** acceptance 2 (a name applied in batch k is offered in batch k+1). Likely in `tests/known_entities.rs`, whose stand-in `claude` already logs every prompt to `prompt.log` (`tests/known_entities.rs:17-30`). Inferred.
- **Tests (kept green):** `tests/known_entities.rs` (332 lines) and `tests/redaction_gate.rs` (397 lines; `refuse_if_left` case at `:149-175`). Cited.
- **Tests (may change):** `tests/known_entities_adversary.rs:335-412` calls `known::terms`, `store_hits`, `merge`, `lines` and `fold` directly, and changes only if those signatures do. Cited calls; the impact is inferred.
- **Tests (may change):** the unit tests in `src/known.rs:215-412` and `src/run.rs:2363-2400` use `merge` and `store_hits`. Cited calls; the impact is inferred.
- **Confidence:** high for the run.rs and known.rs sites, because the story names them and each was read at the stated lines. Medium for the spec/generated/docs ripple, because the digest behaviour was not checked.
- **Would collide with:** any unit touching `src/run.rs` `process_in` between 894 and 1174 (the batch loop, pre-check, redaction arm, apply and entities update). Also anything changing `src/known.rs`, the `UNMAPPED` known-entities comment in `spec/domains/instance.yaml`, the "Known entities" paragraph of `website/docs/operating.md`, or `tests/known_entities.rs`.
- **Safety fact:** today each batch's snapshot sees two things a run-held index will not: (a) nodes and aliases that batch k's `apply-extraction` created or merged, under EKR's canonical name; (b) a foreign writer's commits made during the run. The change is safe only if updating the index from batch k's applied entities produces the same `Store` hits for batch k+1 that a fresh snapshot would. That means the canonical name and type as EKR resolved them, not only the extraction's `(node_type, aliases)` from `extract::entity_names`, which today feeds origin `Run` names (`src/run.rs:1161`). Losing (b) must also be accepted as a contract change in `instance.yaml:1702` and the website doc. Level 2 (`src/known.rs:157-170`, `src/run.rs:1131-1166`), **unproven**.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The index is built from one `ekr snapshot` at the first batch and updated after each apply from the batch's applied entities.
- If `ExtractionReport` carries canonical names, use them; if it does not, record the gap in the test's doc comment and use `extract::entity_names`.
- Acceptance 2's test asserts a `Store`-origin hit ("in the store"), so it does not pass through the `Run`-origin path that exists today.
- A foreign writer's commit during the run is no longer seen; the `instance.yaml:1702-1717` comment and `website/docs/operating.md:202`, `:225-227` say so.
- Hoisting `store.ontology()` out of the loop is in scope.

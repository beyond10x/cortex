---
format: aep.planning-md/3
id: story:spec-standalone-types
kind: story
status: implemented
title: The 1.0 types are in the specification and cortex behaves as before
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/extract.rs
- confidence: inferred
  path: src/home.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/model_map.rs
- confidence: cited
  path: src/ports.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: src/sources.rs
- confidence: cited
  path: src/spec.rs
- confidence: cited
  path: tests/common/mod.rs
- confidence: inferred
  path: tests/conformance.rs
- confidence: cited
  path: tests/e2e.rs
- confidence: cited
  path: tests/spec_compat.rs
- confidence: inferred
  path: website/data/ess
- confidence: inferred
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/reference
- confidence: cited
  path: website/static/schemas
revision: 34
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 31, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 32, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-05T11:47:27Z", actor: "agent:claude", revision: 34, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

The types drafted for 1.0 are part of the specification, the generated code and suite follow them,
and cortex behaves exactly as before for every existing spec file.

## Work

- Land the draft from branch `draft/standalone-spec` (`spec/domains/instance.yaml`: `StoreSpec`,
  `ModelBackend`, `RedactionPolicy`, `StructuredSource`, `RecordMapping`, `SnapshotPolicy`; the two
  `UNMAPPED:` commands stay markers).
- `task generate`; map the new types in `src/model_map.rs` and accept them in `src/spec.rs`.
- No refusals are added for the new settings. Between this story and the feature story that gives a
  setting its behaviour, `main` parses the setting and does not act on it; no release is cut in
  between (`story:release-1-0` is the first release after this epic starts), and
  `story:docs-for-1-0` documents only what has landed. This keeps every feature story out of
  `src/spec.rs`.
- Prepare the seams the feature stories share, so they can run in parallel:
  - move the e2e stand-ins and the `World` fixture (`tests/e2e.rs:9-197`) into `tests/common/mod.rs`;
    each feature story then adds its own `tests/<feature>.rs` instead of editing `tests/e2e.rs`;
  - make a model answer's cost optional (`Option` in `src/extract.rs`); in the budget loop
    (`src/run.rs:161-198`) an answer with no cost consumes no dollar budget, and such a run is
    bounded by the limits that already exist (`policy.max_documents_per_run`, `model.timeout_s`).
    A backend that reports no dollar cost then needs no change to `src/run.rs`.
  - make the reported cost optional end to end: `SourceRan.cost_usd` is `Optional<Decimal>` in the
    draft (`spec/domains/instance.yaml`, the `SourceRan` event), `Report.cost_usd` is
    `Option<f64>` (`src/run.rs:35,198,253`), and `src/ports.rs:266` and `src/main.rs:483,677` pass
    it through. The rule: a run that made no model call reports `0`; a run in which every model
    answer carried a cost reports their sum; a run in which any answer carried no cost reports
    `null`. A missing cost is never written as `0`.
  - plumb the model backend: `Tools` (`src/run.rs:19-22`) gains `codex: PathBuf`, set by a
    `--codex` flag beside `--claude` (`src/main.rs:36-38,173-178`); the `Model` built at
    `src/run.rs:149-153` carries `spec.model.backend` (default `Claude`). `src/extract.rs` answers a
    `Codex` backend with an extraction failure naming `story:codex-model-backend` until that story
    replaces it, so the Codex story edits only `src/extract.rs`.

## Organisation-scale types

Added 2026-10-05 for `epic:organisation-scale-instance` so that every new type lands in this one
story, before any feature code. Applied on top of branch `draft/standalone-spec` in a scratch copy:
`ess specify validate --path spec`: "cortex v1 — 3 file(s), valid"; `ess verify conform
synthesize`: 35 scenarios, 0 refusals. `StructuredSource` takes the `input` union form from this
diff, which `story:structured-source` then implements. `{since}` and `{until}` in a `connectors`
input are placeholders and need no type.

```diff
@@ -98,6 +98,26 @@
       - {name: connection, type: String}
       - {name: input, type: cortex.instance.WebInput}
 
+  - name: cortex.instance.PageStyle
+    kind: enum
+    variants: [PageNumber, Token, Keyset]
+
+  - name: cortex.instance.Paging
+    kind: struct
+    fields:
+      - {name: style, type: cortex.instance.PageStyle}
+      - {name: param, type: String}
+      - {name: next, type: Optional<String>}
+      - {name: max_pages, type: Integer}
+
+  - name: cortex.instance.ChildCall
+    kind: struct
+    fields:
+      - {name: operation, type: String}
+      - {name: input, type: Json}
+      - {name: records, type: String}
+      - {name: paging, type: Optional<cortex.instance.Paging>}
+
   - name: cortex.instance.ConnectorsSource
     kind: struct
     fields:
@@ -109,12 +129,37 @@
       - {name: id, type: String}
       - {name: time, type: Optional<String>}
       - {name: text, type: List<String>}
+      - {name: paging, type: Optional<cortex.instance.Paging>}
+      - {name: child, type: Optional<cortex.instance.ChildCall>}
+
+  - name: cortex.instance.RecordFormat
+    kind: enum
+    variants: [WholeFile, JsonLines, MarkdownSections]
+
+  - name: cortex.instance.RecordFilter
+    kind: struct
+    fields:
+      - {name: field, type: String}
+      - {name: values, type: List<String>}
+      - {name: include, type: Boolean}
+
+  - name: cortex.instance.FileRecords
+    kind: struct
+    fields:
+      - {name: format, type: cortex.instance.RecordFormat}
+      - {name: id, type: String}
+      - {name: time, type: Optional<String>}
+      - {name: author, type: Optional<String>}
+      - {name: text, type: List<String>}
+      - {name: thread, type: Optional<String>}
+      - {name: filters, type: List<cortex.instance.RecordFilter>}
 
   - name: cortex.instance.FilesSource
     kind: struct
     fields:
       - {name: paths, type: List<String>}
       - {name: glob, type: String}
+      - {name: records, type: Optional<cortex.instance.FileRecords>}
 
   - name: cortex.instance.PropertyMapping
     kind: struct
@@ -139,15 +184,39 @@
       - {name: properties, type: List<cortex.instance.PropertyMapping>}
       - {name: relations, type: List<cortex.instance.RelationMapping>}
 
-  - name: cortex.instance.StructuredSource
+  - name: cortex.instance.StructuredConnectors
     kind: struct
     fields:
       - {name: adapter, type: String}
       - {name: connection, type: String}
       - {name: operation, type: String}
       - {name: inputs, type: List<Json>}
+      - {name: paging, type: Optional<cortex.instance.Paging>}
+
+  - name: cortex.instance.StructuredFiles
+    kind: struct
+    fields:
+      - {name: paths, type: List<String>}
+      - {name: glob, type: String}
+
+  - name: cortex.instance.StructuredInput
+    kind: union
+    tag: from
+    variants:
+      connectors: cortex.instance.StructuredConnectors
+      files: cortex.instance.StructuredFiles
+
+  - name: cortex.instance.DropPolicy
+    kind: enum
+    variants: [Keep, Supersede]
+
+  - name: cortex.instance.StructuredSource
+    kind: struct
+    fields:
+      - {name: input, type: cortex.instance.StructuredInput}
       - {name: records, type: String}
       - {name: mapping, type: cortex.instance.RecordMapping}
+      - {name: dropped, type: Optional<cortex.instance.DropPolicy>}
 
   - name: cortex.instance.SourceSettings
     kind: union
@@ -205,7 +274,7 @@
 
   - name: cortex.instance.RedactionClass
     kind: enum
-    variants: [Email, Phone, IpAddress, PaymentCard]
+    variants: [Email, Phone, IpAddress, PaymentCard, Url, Credential, RareName]
 
   - name: cortex.instance.RedactionRule
     kind: struct
@@ -219,6 +288,21 @@
     fields:
       - {name: classes, type: List<cortex.instance.RedactionClass>}
       - {name: rules, type: List<cortex.instance.RedactionRule>}
+      - {name: known_names, type: Optional<List<String>>}
+      - {name: rare_limit, type: Optional<Integer>}
+      - {name: refuse_if_left, type: Optional<List<cortex.instance.RedactionClass>>}
+
+  - name: cortex.instance.GateCheck
+    kind: struct
+    fields:
+      - {name: measure, type: String}
+      - {name: min, type: Optional<Decimal>}
+      - {name: max, type: Optional<Decimal>}
+
+  - name: cortex.instance.RunGate
+    kind: struct
+    fields:
+      - {name: checks, type: List<cortex.instance.GateCheck>}
 
   - name: cortex.instance.SnapshotPolicy
     kind: struct
@@ -250,6 +334,7 @@
       - {name: store, type: Optional<cortex.instance.StoreSpec>}
       - {name: redaction, type: Optional<cortex.instance.RedactionPolicy>}
       - {name: snapshots, type: Optional<cortex.instance.SnapshotPolicy>}
+      - {name: gate, type: Optional<cortex.instance.RunGate>}
 
 entities:
   - name: cortex.instance.Instance
```

## Acceptance

`tests/spec_compat.rs`: `examples/example.yaml`, and the same file with every new field set to its
default (`store: {backend: sqlite}`, `model.backend: Claude`, empty `redaction`, no `snapshots`),
and the same file with the organisation-scale fields at their defaults, all create an instance whose
frozen spec, registry entry and first-run report equal those produced
on `main` at `f2ce283` with stand-in `claude` and `connectors`.

## Scope

Landed 2026-10-05 in `b628e4d` (wave 20261005a, merged `1caa195`); read from `git show --stat b628e4d`.

- **Files, as scoped:** `spec/domains/instance.yaml`, `spec/suite.json`, `generated/` (103 files), `src/model_map.rs`, `src/spec.rs`, `src/extract.rs`, `src/run.rs`, `src/ports.rs`, `src/main.rs`, `tests/e2e.rs`, `tests/common/mod.rs` (new), `tests/spec_compat.rs` (new), `website/docs/reference/ess/cortex-instance.md`, `website/static/schemas/instance-spec.schema.json`
- **Inferred lines, confirmed:** `tests/conformance.rs` (+9/-), `src/sources.rs` (+4), `src/home.rs` (+2) needed edits
- **Inferred line, wrong:** `website/data/ess/` did not change
- **Added beyond the scope:** `tests/fixtures/spec_compat/example.json` (recorded from `f2ce283`, re-recorded independently by the adversary, byte-identical), `tests/cost.rs` and `tests/spec_forms.rs` (adversary cases moved in)
- **Spec change beyond the draft:** the sqlite store is `Optional<cortex.instance.SqliteStore>`, so `store: {backend: sqlite}` as the Acceptance writes it is accepted
- **Left to `story:docs-for-1-0`:** `website/docs/commands.md` (`--codex`, `cost_usd` may be null); patch at `~/.cache/cortex-wave-20261005a/spec/commands-md.patch`, carried into that story
- **Not acted on yet (by design):** store backend, model backend Codex, redaction, snapshots, gate, structured sources, paging, file records
- **Review:** `review-result:adversary-spec-standalone-types-pass-1` (3, fixed) and `-pass-2` (6, fixed; coordinator check: 7 `src/run.rs` mutants each fail the unit suite, `spec/coord-verify/mutant-*.log`)

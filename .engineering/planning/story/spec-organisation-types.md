---
format: aep.planning-md/3
id: story:spec-organisation-types
kind: story
status: archived
title: The organisation-scale types are in the specification and cortex behaves as before
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
revision: 18
transitions:
- {from: "draft", to: "archived", at: "2026-10-05T09:20:50Z", actor: "agent:claude", revision: 10}
---
## Outcome

The types an organisation-scale instance needs are part of the specification, and cortex behaves
exactly as before for every existing spec file.

## The types

Drafted on 2026-10-05 on top of branch `draft/standalone-spec`; a scratch copy with this diff
applied gives `ess specify validate --path spec`: "cortex v1 — 3 file(s), valid" and
`ess verify conform synthesize`: 35 scenarios, 0 refusals. The diff, against that branch:

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

`{since}` and `{until}` in a `connectors` input are placeholders filled from the last successful
run; they need no type.

## Work

- Apply the diff, `task generate`, map the types in `src/model_map.rs`, accept them in
  `src/spec.rs`. As in `story:spec-standalone-types`, no refusals: until a feature story lands,
  `main` parses its fields and does not act on them, and no release is cut in between.

## Acceptance

`tests/spec_compat.rs` still passes unchanged, and a spec setting every new field to its
default creates an instance whose frozen spec, registry entry and first-run report equal the
run without those fields.

## Depends on

`story:spec-standalone-types` (the types extend its draft).

## Scope

`spec/domains/instance.yaml`, `generated/`, `spec/suite.json`, `src/model_map.rs`, `src/spec.rs`,
`tests/spec_compat.rs`, `website/docs/reference/`, `website/static/schemas/`.

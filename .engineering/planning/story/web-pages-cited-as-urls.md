---
format: aep.planning-md/3
id: story:web-pages-cited-as-urls
kind: story
status: draft
title: A fact from a web page cites the page as URL evidence
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:ci-runs-task-check
- depends_on: story:extraction-links-facts
- depends_on: story:store-backend-per-instance
- depends_on: story:adopt-existing-store
- depends_on: story:docs-for-1-0
- depends_on: story:document-time-as-valid-time
- depends_on: story:quality-judge
- depends_on: story:extraction-supersedes
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: examples/example.yaml
- confidence: cited
  path: src/evidence.rs
- confidence: cited
  path: src/extract.rs
- confidence: cited
  path: src/main.rs
- confidence: inferred
  path: tests/common/mod.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: cited
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/spec-file.md
revision: 22
---
## Outcome

A fact taken from a web page cites the page as URL evidence, not as a statement a person made.

## Why it waits

EKR 0.0.30 refuses any extraction evidence source other than `!HumanStatement`
(`extraction-evidence-kind-unsupported`, `crates/ekr-integrate/src/extraction.rs:722`,
`crates/ekr-kernel/src/validate/provenance.rs:129`), although the graph model declares `!Url`
(`crates/ekr-graph/src/evidence.rs:46`). cortex therefore files a page as
`!HumanStatement {identity: <url>}` (`src/evidence.rs`, `AGENTS.md` upstream table). The fix is in
EKR; `upstream-blocker:ekr-url-evidence` holds that.

## Acceptance

With an EKR release that admits URL evidence: cortex writes `!Url` evidence for web documents, the
e2e test asserts the evidence kind, and the `AGENTS.md` upstream row is removed.

## Depends on

`story:ci-runs-task-check` (the workflow it adds pins the EKR binary this story moves),
`story:extraction-links-facts` and `story:document-time-as-valid-time` (both edit `src/extract.rs`;
the last also `src/evidence.rs`), `story:store-backend-per-instance` (both edit `src/main.rs`),
`story:adopt-existing-store` (both edit `tests/conformance.rs`), and `story:docs-for-1-0` (both
edit the four docs pages; the docs story writes the URL-evidence limit as open, and this story
closes it).

`story:codex-model-backend` also edits `src/extract.rs`, but nothing of it is on `main`: its unit
is archived until `decision-blocker:codex-exec-keeps-shell` clears. The edge was removed on
2026-10-08 so that this story waits only on its own upstream blocker; whichever of the two lands
second rebases onto the first.

## Scope

- `src/evidence.rs`; `src/extract.rs` (calls `evidence::identity` at line 138, cites `issued.id`
  in `web_page_facts` at line 311, builds the evidence list at line 350).
- The EKR pin: `src/main.rs:89` (default version), `tests/common/mod.rs` (the e2e helpers holding
  the pin at `tests/e2e.rs:14,18,128` today move there with `story:spec-standalone-types`),
  `tests/conformance.rs:45`, `examples/example.yaml:5`, `AGENTS.md:108-109` (the upstream rows),
  `.github/workflows/check.yml`.
- The docs that name the pin or the open limit: `website/docs/commands.md:59`,
  `website/docs/quickstart.md:28`, `website/docs/spec-file.md:23`, `website/docs/limits.md:25`.

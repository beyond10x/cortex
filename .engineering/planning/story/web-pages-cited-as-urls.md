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
revision: 2
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

`story:ci-runs-task-check` (the workflow it adds pins the EKR binary this story moves) and
`story:extraction-links-facts` (both edit `src/extract.rs`).

## Scope

- `src/evidence.rs`; `src/extract.rs` (calls `evidence::identity` at line 138, cites `issued.id`
  in `web_page_facts` at line 311, builds the evidence list at line 350).
- The EKR pin: `src/main.rs:89` (default version), `tests/e2e.rs:14,18,128`, `AGENTS.md:54,68-69`,
  and `.github/workflows/check.yml` once `story:ci-runs-task-check` adds it.

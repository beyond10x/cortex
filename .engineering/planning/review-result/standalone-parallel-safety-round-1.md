---
format: aep.planning-md/3
id: review-result:standalone-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, standalone 1.0, round 1
relations:
- reviews: epic:standalone-1-0
- reviews: story:spec-standalone-types
- reviews: story:store-backend-per-instance
- reviews: story:redaction-before-model
- reviews: story:structured-source
- reviews: story:codex-model-backend
- reviews: story:run-snapshots
- reviews: story:adopt-existing-store
- reviews: story:release-pipeline
- reviews: story:docs-for-1-0
- reviews: story:release-1-0
- reviews: story:web-pages-cited-as-urls
- reviews: story:first-web-instance
revision: 1
---
needs-revision
story:spec-standalone-types — the refusals for the five non-default settings will land in the one validator `check` in `src/spec.rs:49-95` (inferred: the refusal does not exist yet and `spec.rs` is where `create` validates), and `store-backend-per-instance`, `redaction-before-model` and `codex-model-backend` each remove their own refusal there in the same wave, yet no scope lists `src/spec.rs`; the body must either name the file as shared (ordering edge) or put each refusal in its own seam — .engineering/planning/story/spec-standalone-types.md:47
story:codex-model-backend — the per-run call and token limit that replaces the dollar budget has to live in the budget loop at `src/run.rs:161-198` (cited in the tree; `Model::ask` takes `remaining`), but the scope is `src/extract.rs` only, and `story:redaction-before-model` (same wave 2, scope cites `src/run.rs`) and `story:structured-source` are unordered against it (edge or split needed) — .engineering/planning/story/codex-model-backend.md:27
story:store-backend-per-instance — "run `ekr postgres-schema` once, then seed" belongs in `seed_instance` at `src/main.rs:327-363` (cited in the tree), which is not in the scope, and `story:run-snapshots`, `story:web-pages-cited-as-urls` and `story:adopt-existing-store` all scope `src/main.rs` (edge or split needed; the last is already ordered) — .engineering/planning/story/store-backend-per-instance.md:29
story:web-pages-cited-as-urls — moving the EKR pin also edits `tests/conformance.rs:45` (`version: "0.0.30"`, cited in the tree), which is absent from its scope but is in `story:run-snapshots` and `story:adopt-existing-store` (no edge to either); and its `tests/e2e.rs:14,18,128` will have moved to `tests/common/mod.rs` once `spec-standalone-types` lands — .engineering/planning/story/web-pages-cited-as-urls.md:55
story:first-web-instance — its Concurrency section names only `story:web-pages-cited-as-urls`, but `aep plan artifact waves` puts it in wave 5 beside `story:run-snapshots` (and `story:adopt-existing-store` in wave 6), which rewrite `src/run.rs`, `src/home.rs`, `src/state.rs` and `generated/` during the 7-day window, and the epic says the run must not have "the seen-state format or generated types change under it"; whether the instance runs a build of this tree is inferred, so the body must say which stories land before day 1 or after day 8 — .engineering/planning/story/first-web-instance.md:57

What I read: 14 stories and both epics via `aep plan artifact show`, plus `aep plan artifact graph` and `aep plan artifact waves`. I also read `src/spec.rs`, `src/main.rs`, `src/run.rs`, `src/instance.rs`, `src/ekr.rs`, `Cargo.toml`, `tests/` and `spec/domains/instance.yaml` in the tree. Surfaces established: 13 stories cited, 1 inferred only (`story:first-web-instance`, examples files that do not exist yet), 0 unplaceable.

What I could not establish:
- The collisions `waves` lists between stories with declared scope (`AGENTS.md` for redaction, release-pipeline and web-pages; `README.md` for docs and timer; `src/main.rs` for adopt, run-snapshots and web-pages; `src/run.rs`, `src/state.rs` and `src/home.rs` for timer against redaction, structured-source, run-snapshots and adopt) are all separated by wave placement. I raised no finding on them. Wave placement is computed from scope and is not an edge, so it holds only while the scopes stay as drafted.
- `src/main.rs` is touched by several stories in different regions (the Command enum, `seed_instance`, line 89). Whether those merge cleanly is unclear without trying.
- Out of my lane, not raised: `story:codex-model-backend` names a per-run call and token limit, but `ModelSpec` in `spec/domains/instance.yaml` has no field for one (acceptance or design critic).
- `story:ci-runs-task-check` and `story:extraction-links-facts` are outside the set and `waves` marks them unassessed.

```findings
[
  {"file": ".engineering/planning/story/spec-standalone-types.md", "line": 47, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the refusals for the five non-default settings land in one validator (src/spec.rs check, inferred; the refusal does not exist yet) and store-backend-per-instance, redaction-before-model and codex-model-backend each remove their own there in the same wave, but no scope lists src/spec.rs; name the shared file with an ordering edge or give each refusal its own seam"},
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 27, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the per-run call and token limit replacing the dollar budget belongs in the budget loop at src/run.rs:161-198 (cited in the tree) but the scope is src/extract.rs only; redaction-before-model (same wave) and structured-source scope src/run.rs and are unordered against it; add the file and an ordering edge or split the surface"},
  {"file": ".engineering/planning/story/store-backend-per-instance.md", "line": 29, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "running ekr postgres-schema at create belongs in seed_instance at src/main.rs:327-363 (cited in the tree), absent from the scope, while run-snapshots, web-pages-cited-as-urls and adopt-existing-store scope src/main.rs; add the file and an ordering edge or split the surface"},
  {"file": ".engineering/planning/story/web-pages-cited-as-urls.md", "line": 55, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "moving the EKR pin also edits tests/conformance.rs:45 (cited in the tree), absent from the scope but in run-snapshots and adopt-existing-store with no edge to either, and its tests/e2e.rs:14,18,128 move to tests/common/mod.rs once spec-standalone-types lands"},
  {"file": ".engineering/planning/story/first-web-instance.md", "line": 57, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the Concurrency section names only web-pages-cited-as-urls, but waves places this 7-day run in wave 5 beside run-snapshots (adopt-existing-store follows in wave 6), which rewrite src/run.rs, src/home.rs, src/state.rs and generated/ during the window; that the instance runs a build of this tree is inferred; say which stories land before day 1 or after day 8"}
]
```

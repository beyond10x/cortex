---
format: aep.planning-md/3
id: review-result:standalone-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, standalone 1.0, round 2
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
- reviews: story:timer-runs-unattended
revision: 1
---
needs-revision
story:codex-model-backend — the backend choice and the null cost need `src/run.rs:149-153` (the `Model` built from `tools.claude`, from `Tools` at `src/run.rs:19-22`), `src/main.rs:36-38,173-178` (the `--claude` flag), and `Report.cost_usd: f64` at `src/run.rs:35,253` with `src/ports.rs:266` and `src/main.rs:483`; none is in its scope of `src/extract.rs` and `tests/codex.rs`, and `story:redaction-before-model`, `story:structured-source` and `story:run-snapshots` (all `src/run.rs`) and `story:timer-runs-unattended` (`src/ports.rs:260`) are unordered against it, so the body must name those files and an ordering edge or move them into `story:spec-standalone-types` (surfaces cited in the tree; that Codex must edit them is inferred from the acceptance `"cost_usd": null`) — .engineering/planning/story/codex-model-backend.md:47
story:store-backend-per-instance — a postgres instance still gets a viewer unit that hardcodes `EKR_BACKEND=sqlite` and the `store.sqlite` path (`src/schedule.rs:127-146`, `install_view`) and an MCP line that does the same (`src/main.rs:815`, outside "`seed_instance` only"), and `src/schedule.rs` is cited scope of `story:timer-runs-unattended`, which has no edge to this story, so the body must add the two sites and an ordering edge, or limit the outcome to the CLI path (surfaces cited in the tree; the need follows from `create` starting a viewer, `src/main.rs:45`) — .engineering/planning/story/store-backend-per-instance.md:54
story:run-snapshots — "stops the viewer, restores, restarts it" needs a stop/start of the viewer unit, and `Systemd` in `src/schedule.rs` has only `install_view`, `set_source_timer` and `remove_instance`, so it needs a new method there; `src/schedule.rs` is absent from the scope, and `story:timer-runs-unattended` (cited scope `src/schedule.rs`) is unordered against this story (surface cited in the tree; the new method is inferred), so the body must add the file with an ordering edge or split the viewer control into its own seam — .engineering/planning/story/run-snapshots.md:66
story:web-pages-cited-as-urls — moving the EKR pin also edits `website/docs/commands.md:59`, `website/docs/quickstart.md:28`, `website/docs/spec-file.md:23` and `website/docs/limits.md:25` (the "cited as a statement" limit), and `examples/example.yaml:5`; `story:docs-for-1-0` scopes the four docs files and writes the URL-evidence limit as open, both land in wave 7 and no edge joins them, so the body must add the files and an ordering edge, or split the doc rows (cited, both bodies and the tree) — .engineering/planning/story/web-pages-cited-as-urls.md:58

What I read: 14 stories, both epics and `review-result:standalone-parallel-safety-round-1`, via `aep plan artifact show` on each, plus `aep plan artifact graph` and `aep plan artifact waves`. I also read `Cargo.toml`, `src/run.rs`, `src/extract.rs`, `src/spec.rs`, `src/ports.rs`, `src/schedule.rs`, `src/instance.rs`, and `src/main.rs` at lines 36-38, 80-95, 165-185, 320-365 and 800-825. I grepped the `0.0.30` pin and `sqlite` through the tree. Surfaces established: 13 stories cited, 1 inferred only (`story:first-web-instance`, example files that do not exist yet), 0 unplaceable.

Round 1 status:
- Fixed and not repeated: the `src/spec.rs` refusals (`story:spec-standalone-types` now adds none) and the budget loop (the spec story now owns it).
- Fixed and not repeated: the `src/main.rs` and `tests/conformance.rs` collisions (edges now run to store-backend, run-snapshots and adopt).
- Fixed and not repeated: `story:first-web-instance`'s Concurrency section now names the one day-1 binary.
- Still holding: the `story:codex-model-backend` finding, in part. Its budget-loop half is fixed; the `src/run.rs:149-153` and null-cost half is not.

What I could not establish:
- Collisions that `waves` separates by wave but no edge records, unraised as in round 1: `AGENTS.md` (`story:release-pipeline` against `story:redaction-before-model` and `story:web-pages-cited-as-urls`), `README.md` (`story:docs-for-1-0` against `story:timer-runs-unattended`), and `src/run.rs`, `src/state.rs`, `src/home.rs` and `src/ports.rs` (`story:timer-runs-unattended` against redaction, structured-source, run-snapshots and adopt). They hold only while the scopes stay as drafted.
- Whether `story:run-snapshots` needs a new dependency for SQLite's online backup: `Cargo.toml` has no SQLite crate and the body names no mechanism. If it adds one, `Cargo.toml` and `Cargo.lock` collide with `story:release-pipeline`, which has no edge to it.
- Whether `story:seen-documents-modelled`'s generated-type change forces edits in `src/run.rs` or `src/ports.rs`, which it does not scope.
- Out of my lane: `story:web-pages-cited-as-urls` cites `AGENTS.md:54,68-69`, but the upstream rows now sit at `AGENTS.md:108-109`.

```findings
[
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 47, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the backend choice and the null cost need src/run.rs:149-153 (Model built from tools.claude), src/main.rs:36-38,173-178 (--claude flag) and Report.cost_usd at src/run.rs:35,253 with src/ports.rs:266 and src/main.rs:483, none in the scope of src/extract.rs and tests/codex.rs; redaction-before-model, structured-source and run-snapshots (src/run.rs) and timer-runs-unattended (src/ports.rs:260) are unordered against it; name the files and an ordering edge or move them into spec-standalone-types (surfaces cited in the tree; that Codex must edit them is inferred from the acceptance cost_usd null)"},
  {"file": ".engineering/planning/story/store-backend-per-instance.md", "line": 54, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "a postgres instance still gets a viewer unit hardcoding EKR_BACKEND=sqlite and the store.sqlite path (src/schedule.rs:127-146) and an MCP line doing the same (src/main.rs:815, outside seed_instance only); src/schedule.rs is cited scope of timer-runs-unattended, which has no edge to this story; add the sites and an ordering edge or limit the outcome (surfaces cited in the tree; the need follows from create starting a viewer, inferred)"},
  {"file": ".engineering/planning/story/run-snapshots.md", "line": 66, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "stopping and restarting the viewer needs a new method on Systemd in src/schedule.rs (it has only install_view, set_source_timer and remove_instance); the file is absent from the scope and timer-runs-unattended (cited scope src/schedule.rs) is unordered against this story; add the file with an ordering edge or split the viewer control into its own seam (surface cited in the tree; the new method is inferred)"},
  {"file": ".engineering/planning/story/web-pages-cited-as-urls.md", "line": 58, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "moving the EKR pin also edits website/docs/commands.md:59, quickstart.md:28, spec-file.md:23, limits.md:25 and examples/example.yaml:5; docs-for-1-0 scopes the four docs files and writes the URL-evidence limit as open, both are wave 7 and no edge joins them; add the files and an ordering edge or split the doc rows (cited, both bodies and the tree)"}
]
```

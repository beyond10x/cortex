---
format: aep.planning-md/3
id: story:model-through-gateway
kind: story
status: implemented
title: Extraction runs through a model gateway the spec names, with no interactive sign-in
summary: model.gateway names a base URL and a token variable; cortex passes exactly that set to claude in every run
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T03:36:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An instance's spec can name a model gateway: `model.gateway` gives the base URL and the name of the
environment variable that holds the gateway's token. cortex passes exactly that set to `claude -p`
(`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and `CLAUDE_CODE_MAX_CONTEXT_TOKENS` when the spec
sets it), from an interactive run, a timer unit and a runner that calls `cortex run` directly alike,
and still removes `ANTHROPIC_API_KEY`. Extraction can then run unattended against an open model
behind a gateway, with no interactive sign-in.

## Why

A downstream deployment runs extraction against an open model behind a gateway and needs it to work
without an interactive `claude` sign-in (gap 4 of `epic:periodic-collection-reconcile`). Today it
works only by accident. Verified in this tree:

- `ask_claude` (`src/extract.rs`, the `Command` near `.env_remove("ANTHROPIC_API_KEY")`) inherits
  cortex's whole environment and removes only `ANTHROPIC_API_KEY`, so `ANTHROPIC_BASE_URL` and
  `ANTHROPIC_AUTH_TOKEN` reach `claude` when they are in cortex's environment. No test sets or reads
  them; the operating page (`website/docs/operating.md`, "`ANTHROPIC_API_KEY` is removed") and
  `website/docs/limits.md` promise only the sign-in.
- The units cortex writes carry only `PATH`, `HOME`, `XDG_*` and `CONNECTORS_*`
  (`src/schedule.rs`, `environment_values`), so a timer run sees a gateway variable only through
  the user manager's environment.
- `claude` runs with `--setting-sources ""`, so a settings file's `env` block is never read; the
  environment is the only route. `--model <model.name>` is always passed, so the gateway's model is
  named in the spec.

## Domain relations

No new noun: an optional `gateway` value on `cortex.instance.Model` (`spec/domains/instance.yaml`)
with `base_url` (String) and `token_env` (String, the variable's name, never its value), and an
optional Integer `max_context_tokens`.

## Work

- Spec first: add the optional `gateway` to `cortex.instance.Model` with a comment stating the
  pass-through set and that `ANTHROPIC_API_KEY` is always removed; validate with the pinned `ess`,
  run the conformance suite, `task generate`. Then the code.
- `src/extract.rs`: with a gateway, set `ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN` (read from
  `token_env` at call time) and `CLAUDE_CODE_MAX_CONTEXT_TOKENS` on the `claude` command; a missing
  or empty token variable fails the call with a reason naming the variable, before `claude` runs.
  Without a gateway, remove `ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN` as well, so an instance
  without one never reaches a gateway by accident.
- `src/schedule.rs`: a source unit carries the variable `token_env` names, by reference to the
  user manager's environment or an `EnvironmentFile`, never its value in the unit file; the
  operating page says which.
- Tests: a stand-in `claude` that records the environment it saw; docs: the spec-file and operating
  pages, and `limits.md`'s line on the sign-in.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With `model.gateway: {base_url: "http://gw.test", token_env: GW_TOKEN}` and `GW_TOKEN=t1` in the
  environment, a run's stand-in `claude` sees `ANTHROPIC_BASE_URL=http://gw.test` and
  `ANTHROPIC_AUTH_TOKEN=t1`, and no `ANTHROPIC_API_KEY` although one was set.
- With the gateway and `GW_TOKEN` unset, the run fails with a reason naming `GW_TOKEN`, and the
  stand-in `claude` is never started.
- Without a gateway and with `ANTHROPIC_BASE_URL` set in cortex's environment, the stand-in
  `claude` sees no `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN` or `ANTHROPIC_API_KEY`.
- No unit file cortex writes contains the token's value.

## Not in this story

- A model backend other than `claude -p` (`story:codex-model-backend`).
- A budget for a model whose cost `claude` cannot price (`story:token-budget`).
- Entering a real token or any spend it carries: the operator's.

---
format: aep.planning-md/3
id: story:eval-proxy-bounds
kind: story
status: implemented
title: The eval MCP proxy bounds its sockets and its header reads
summary: Closed connections are pruned from the proxy's open list; request and header lines have a size cap checked before the bearer check
tags:
- review-20261030
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/ask/proxy.rs
- confidence: inferred
  path: tests/ask_redaction_security.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:10Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

- Each accepted connection's socket clone is pushed into `open` and never pruned until the case
  ends (`src/ask/proxy.rs:56-68`, `:124-127`); at the descriptor limit `accept` fails and the loop
  only sleeps (`:142`) (R1-07);
- request and header lines are read without a cap, and the bearer check runs after the whole
  header block (`src/ask/proxy.rs:182-222`; R1-08);
- the proxy's doc comment says tool results are shown as the service shows them, but it passes
  them uncut (`src/ask/proxy.rs:8-9`, `:29`, `:278`; R1-09 doc nit).

## Acceptance

1. 2,000 sequential connections to the proxy leave its open list bounded (a count seam, new test).
2. A header line over the cap is refused before any further read.
3. The doc comment says what the proxy does.

## Findings it closes

R1-07, R1-08, R1-09.

## Scope

Derived 2026-10-30 by `story-scoper` against `b61a56c8` (src/, tests/, spec/ and generated/ identical to `a870a628`). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/ask/proxy.rs` — cited; the change stays in this one file
- **Citation check, R1-07:** `:56-68` is `Stopper` and its `Drop` (doc comment at 54-55, the closing brace at 69). `:124-127` is the `accept` arm that pushes `stream.try_clone()` into `open`. `:142` is `Err(_) => sleep(10ms)`. All confirmed — cited
- **Citation check, R1-08:** `:182-222` confirmed. `handle` reads `request_line` at 184-187 and the header loop at 190-209 with an unbounded `read_line`. The bearer check is at 213-215, after the whole header block. `MAX_REQUEST` covers only the body length, at 222-224 — cited
- **Citation check, R1-09:** confirmed. `:8-9` is the module doc ("Each tool result is shown as the ask service shows it"). `:28-29` is the `NO_CUT` doc and constant. `:278` is `shown(Some(batch), &text, NO_CUT)`. `shown` in `src/ask/mod.rs:319` cuts to `max`, so the proxy never cuts — cited
- **Symbols edited:** `serve` (`src/ask/proxy.rs:77-153`, the accept loop at 121-145 and the per-connection spawn at 135-140), `Stopper` (54-69), `handle` (173-245), the module doc (1-11) and `NO_CUT` (28-29). A new header-size constant goes next to `MAX_REQUEST` (25-26) — inferred from the acceptance
- **Pruning shape:** pruning needs an identity per connection, for example a keyed map in place of `Mutex<Vec<TcpStream>>` at line 115, with the entry removed after `handle` returns at line 139. `Stopper.open` (line 58) changes type with it — inferred
- **Count seam:** acceptance 1 asks for "a count seam". The seam is not designed yet. It is either a `pub` or `#[doc(hidden)]` count on `Served` (32-35), or a `#[cfg(test)]` module in `proxy.rs`, which has none today — inferred
- **Tests:** `tests/ask_redaction_security.rs` gets the 2,000-connection test and the oversized-header test. It already drives `proxy::serve` at :115 and :153 and has the `url_of` and `post` helpers at :77-107 — inferred
- **Callers not edited:** `src/eval.rs:551-575` reads only `served.answer` and `served.batch`, so a new field on `Served` does not touch it. `src/extract.rs:555` uses only `proxy::token`. Both are edited only if `serve`'s signature changes — cited (callers) / inferred (not edited)
- **Spec / generated:** none. `spec/domains/ask.yaml` and `instance.yaml` do not model the proxy's HTTP surface, and `spec/suite.json` does not reference `ask_redaction_security` — cited (grep)
- **Documents:** the doc comment in `proxy.rs` only. `AGENTS.md:163` says the proxy redacts "as the ask service's tool loop does". It says nothing about cutting, so it does not need to change — cited
- **Confidence:** high. The story names the defect sites, and each one checks out line for line in the current tree
- **Would collide with:** any unit touching `src/ask/proxy.rs` (the eval `mcp` runner's redacting server) or adding cases to `tests/ask_redaction_security.rs`
- **Safety fact:** the new header-line cap must sit above what the `claude` CLI's streamable-HTTP MCP client actually sends. It is a wire format read by a client this repo does not control, and a cap below it turns every redacted eval case into a refused request. Proof level 1, stated; nothing in the tree records `claude`'s header sizes — unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- Header block cap 64 KiB.
- R1-09 is a doc fix only.
- The count seam is a `#[doc(hidden)] pub fn open_connections` on `Served`.

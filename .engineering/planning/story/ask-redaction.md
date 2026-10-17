---
format: aep.planning-md/3
id: story:ask-redaction
kind: story
status: active
title: The ask service and eval runs show the model only redacted store contents
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-service
- depends_on: story:eval-run
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T03:58:48Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T03:58:48Z", actor: "human:timo", revision: 3}
---
## Story

The ask service (`story:ask-service`) and the `mcp` runner of an eval run (`story:eval-run`) send
store contents to the model: every tool result of the tool loop over `ekr mcp` is text from the
instance's store, which holds the original source text. Extraction pseudonymises what the model is
shown through the instance's `redaction` policy (`src/redact.rs`); these two paths do not. This
story applies the same policy to them, so the instance's policy holds wherever the model reads its
store. It lands before the ask service ships in a release.

## Behaviour

- One question (one `POST /ask`, one eval case under the `mcp` runner) is one redaction batch: one
  placeholder mapping, held in memory for that question only.
- Each tool result is redacted before the model is given it, with the instance's policy: its
  classes, `known_names`, rules and `rare_limit`. The question text is redacted too.
- The model's answer is restored before the caller sees it: every placeholder that has a value is
  replaced by it. In the stream, a `delta` that ends inside a possible placeholder holds that tail
  back until the next `delta` or `done` decides it, so no client sees half a placeholder. An
  unrestored placeholder stays as it is and is counted in `done`.
- A tool result in which a class the policy's `refuse_if_left` names is still detected is not
  given to the model: the model gets a tool error saying the result was withheld by policy, and the
  stream reports a `tool` event with outcome `withheld`. The question goes on.
- Credentials and rules with a `replacement` are replaced irreversibly, as in extraction.
- An instance with no `redaction` policy sends tool results unchanged (today's behaviour).

## Acceptance

1. A stand-in model test of `ask serve` over a store whose document holds an email address and a
   known name: the request the stand-in model receives holds `[Email-1]` and `[Name-1]` and neither
   value; the streamed answer that echoes both placeholders reaches the client with the original
   values, including when a placeholder is split across two deltas.
2. The same store with `refuse_if_left: [Email]` and a rule that leaves an address the detector
   still finds: the model receives a withheld tool error, the stream carries a `tool` event with
   outcome `withheld`, and the answer completes.
3. An eval run with the `mcp` runner over that store: the stand-in model sees no original value,
   and the case's scored answer holds the restored values.
4. An instance with no `redaction` policy: the stand-in model receives the tool result byte for
   byte (a regression test).
5. The specification carries the `withheld` outcome and the unrestored count; `ess specify
   validate` and the conformance suite are green.

## Scope

`src/ask/` (tool loop and stream), `src/eval.rs` (`mcp` runner), `src/redact.rs` (a batch usable
across tool calls, if it is not already), `spec/domains/ask.yaml`, `website/docs` (the limit the
ask service documents today is removed), tests beside the ask and eval tests.

---
format: aep.planning-md/3
id: story:epoch-window-placeholders
kind: story
status: active
title: A source's inputs can carry the run's window as epoch seconds
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T03:31:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-15T03:31:24Z", actor: "human:timo", revision: 3}
---
## Outcome

A source's `inputs` can carry the run's window as epoch seconds: `{since_epoch}` and
`{until_epoch}` render the same instants as `{since}` and `{until}`, as a string of whole seconds
since the Unix epoch, wherever those two are rendered. A chat-history API that takes its lower bound
as epoch seconds can then be windowed. Minimal example, on a `kind: connectors` source:

```yaml
inputs:
  - {channel: "C1", oldest: "{since_epoch}"}
```

## Why

A downstream consumer reads a chat-history operation whose lower bound (`oldest`) is epoch seconds,
not RFC 3339, so its source cannot be windowed today. Verified in this tree:

- `with_window` replaces only `{since}` and `{until}` in every string value of an input, each by
  `rfc3339(ms)` (`src/sources.rs:126-142`). `rfc3339` cuts to the second
  (`src/sources.rs:103-124`, `ms.div_euclid(1000)`), so both ends of a window are whole seconds.
- The window is filled into the inputs of a `kind: connectors` source (`src/sources.rs:525`) and of
  a `kind: structured` source read `from: connectors` (`src/sources.rs:935`). Child, link and
  compare inputs are filled from records by `{a.b}` placeholders (`src/sources.rs:144`, `:882`),
  not from the window.
- An unknown placeholder is not refused today: `with_window` uses plain string replacement, so
  `"{since_epoch}"` (which does not contain `{since}`) reaches Connectors as the literal text
  `{since_epoch}`. Nothing in `src/model_map.rs` or the spec checks placeholders in `inputs`.
- `spec/domains/instance.yaml` types `inputs` as `List<Json>` (`ConnectorsSource` at `:125-137`,
  `StructuredConnectors` at `:217-226`) and states nothing about the window placeholders; the rule
  lives in the code, the unit test
  `the_window_fills_since_and_until_in_every_string_of_an_input` (`src/sources.rs:1565`), the
  end-to-end tests in `tests/connectors_walks.rs` (`:157`, `:574`) and
  `website/docs/spec-file.md:132`, `:143-146`.

## Domain relations

No new noun: the placeholders are text inside the `inputs` of `cortex.instance.ConnectorsSource`
and `cortex.instance.StructuredConnectors` (`spec/domains/instance.yaml:125-137`, `:217-226`),
values of a `Source`'s settings owned by `cortex.instance.Instance` through its `sources` relation.

## Work

- Spec first: in `spec/domains/instance.yaml`, state on the `inputs` field of
  `cortex.instance.ConnectorsSource` and `cortex.instance.StructuredConnectors` the four window
  placeholders and their renderings (`{since}`, `{until}` as RFC 3339; `{since_epoch}`,
  `{until_epoch}` as whole epoch seconds), as a comment or an `UNMAPPED:` marker where ESS cannot
  state a rule on the text of a `Json` value; run `ess specify validate` and the conformance suite
  green, then `task generate`. Only then change the code.
- `src/sources.rs` (`with_window`): also replace `{since_epoch}` by `since_ms.div_euclid(1000)` and
  `{until_epoch}` by `until_ms.div_euclid(1000)`, as decimal strings. Decision: whole seconds, no
  fraction, because the window is already cut to the second, so `{since_epoch}` and `{since}` name
  the same instant; an API with fractional timestamps (such as `1790000000.000100`) accepts a whole
  second as a lower bound. The string stays a string: a JSON number is not produced.
- Extend the unit test at `src/sources.rs:1565` and add an end-to-end case to
  `tests/connectors_walks.rs`.
- Document both placeholders in `website/docs/spec-file.md` (the `inputs` row at `:132` and the
  window paragraph at `:143`).

## Acceptance

- `ess specify validate` passes with the placeholders stated on both `inputs` fields, and
  `task drift` is clean after `task generate`.
- A unit test: for `Window::of_run(86_400_000, None, None, None, 1)`, `with_window` turns
  `{"channel": "C1", "oldest": "{since_epoch}", "latest": "{until_epoch}"}` into
  `{"channel": "C1", "oldest": "0", "latest": "86400"}`, and a window whose `since_ms` is
  `1_790_000_000_999` renders `"1790000000"`, the same second `{since}` renders.
- End to end: a source with `inputs: [{channel: "C1", oldest: "{since_epoch}"}]` whose state holds a
  last successful run at a known instant passes `oldest: "<that instant less 5 minutes, in epoch
  seconds>"` to the stand-in `connectors`.
- An input string with an unknown placeholder (for example `"{since_ms}"`) is passed to Connectors
  unchanged, as today; this story adds no refusal.

## Not in this story

Refusing unknown window-shaped placeholders at spec load (today they pass through as text; a
separate story if wanted); other renderings (milliseconds, fractional seconds, a date alone);
placeholders in child, link or compare inputs.

---
format: aep.planning-md/3
id: story:ask-canvas-blocks
kind: story
status: draft
title: An answer reaches the client as typed blocks rendered through an allow-list
summary: Typed canvas blocks (paragraph, markdown, code, source card, figure, image link) with allow-list rendering
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-figure-event
revision: 1
---
## Outcome

An answer of the ask service reaches the client as a sequence of typed blocks, not free text:
`paragraph`, `markdown`, `code` (with its language), `source_card` (a store item the turn saw),
`figure` (the figure event's block) and `image_link` (an image shown only as a link). cortex renders
each block through an allow-list, so a client can show the answer without trusting the model's
markup:

- raw HTML in any block stays text, escaped;
- a link survives only when its scheme is `http` or `https`, or it is same-origin (a relative
  path); every other link is rendered as text;
- each block kind has a size cap; a block over its cap is cut at a character boundary and marked
  truncated;
- `code` is highlighted by cortex's own small highlighter with a fixed class set; removing the
  highlighter's markup from its output gives back the input byte for byte;
- a `source_card` is built only from store facts a tool of the same turn returned; its link is
  rebuilt from the spec's configured base URL and the item's identity, never copied from store
  text or model output.

## Why

A downstream deployment renders answers in a canvas of its own and asked for a typed block model
beside the figure event, so it can drop its own sanitizer. Model output and store text are
untrusted; rendering by an allow-list in cortex keeps one place that decides what markup and which
links reach a reader.

## Domain relations

- `cortex.ask.AnswerBlock`, a `kind`-tagged union of the six kinds above, each with its
  `sequence` in the answer. Relation: `cortex.ask.AnswerRecord` owns many `AnswerBlock` via
  `answer_id`; a `figure` block references `cortex.ask.FigureRecord` by `sequence`.
- `cortex.ask.SourceCard`: `node_id`, `title`, `url` (rebuilt), `evidence` ids. Relation:
  references the store node the turn's tool call returned.
- `AskSpec` gains `card_base_url` and per-kind size caps.
- The event `cortex.ask.AskBlock` joins `cortex.ask.AskEvent`.

## Work

- Spec first: the union, the card, the event, the spec fields; validate, conformance,
  `task generate`. Then the code: the block parser over the model's answer, the renderer and the
  highlighter in their own module, the event in the stream, blocks kept with the answer record.
- Docs: the service page (block kinds, the allow-list, an example stream).

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- An answer whose text carries `<script>`, an `<img onerror>` and a `javascript:` link reaches the
  client with all three as escaped text and no live link.
- A `code` block in each supported language: stripping the highlighter's markup from its output
  equals the input byte for byte (a property test over arbitrary input included), and every class
  it emits is in the fixed set.
- A `source_card` for a node no tool of the turn returned is refused; one for a returned node
  carries the URL `card_base_url` plus the node's identity, also when the store text holds another
  URL.
- A block over its kind's cap is cut and marked truncated; the stream stays valid.

## Not in this story

- The figure tool, renderer and event (`story:figure-svg-renderer`, `story:ask-figure-tool`,
  `story:ask-figure-event`).
- Report downloads (`story:ask-report-download`).

---
format: aep.planning-md/3
id: story:figure-svg-renderer
kind: story
status: implemented
title: cortex draws figures as allow-listed, byte-stable SVG from a typed spec
summary: A renderer for bar charts, timelines and relationship graphs with no script, no foreignObject, no handlers, no external href, and a size cap
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T15:56:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T15:56:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T19:51:11Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

cortex draws a figure from a typed figure spec as SVG with its own code: a bar chart, a timeline
or a relationship graph. The output uses only an allow-list of elements and attributes (`svg`,
`g`, `rect`, `line`, `path`, `circle`, `text`, `title`, with geometry, fill, stroke and text
attributes): no `script`, no `foreignObject`, no event-handler attribute, no `href` or
`xlink:href` to anything outside the document, no `style` element. The same spec always gives the
same bytes, and an SVG over a fixed size cap is refused rather than cut.

## Why

A downstream deployment wants the ask service to answer with figures, and moves its ask
orchestration onto cortex only once a figure cannot carry content a model wrote. An SVG a model
writes is markup a browser runs: the only safe SVG is one cortex drew from typed values.

## Domain relations

New types in `spec/domains/ask.yaml`: `cortex.ask.FigureSpec` (a union of `BarChart`,
`Timeline`, `RelationshipGraph`), each with a title and its typed series: bars as label and
count, timeline entries as instant and label, graph nodes and edges as ids and labels. Labels are
text, escaped on output; ids are store node or evidence ids.

## Work

- Spec first: the figure types, validate, `task generate`, conformance.
- A renderer module (`src/ask/figure.rs`) from spec to SVG, with fixed layout rules and no
  dependency that emits markup.
- A size cap (bytes, and a cap on bars, entries, nodes and edges) stated on the service page.

## Acceptance

- For each figure kind, a fixed spec renders to bytes equal to a committed fixture, twice.
- A property test over generated specs (labels holding `<`, `&`, `"`, `javascript:`, `on*=`,
  control characters) finds no element or attribute outside the allow-list in any output.
- A spec over the cap is refused with a typed error; no partial SVG is returned.

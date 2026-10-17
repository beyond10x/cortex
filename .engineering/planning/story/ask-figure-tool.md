---
format: aep.planning-md/3
id: story:ask-figure-tool
kind: story
status: draft
title: The model asks for a figure; cortex counts its values from the turn's tool results
summary: A figure tool with a typed request; values counted by cortex, never typed by the model, never SVG from the model
relations:
- decomposes: epic:instance-answers-questions
- depends_on: story:figure-svg-renderer
- depends_on: story:ask-service
revision: 2
---
## Outcome

The ask service offers the model a `figure` tool. The model calls it with a typed figure request:
the kind (bar chart, timeline, relationship graph), a title, and which of the turn's tool results
to count and how (group by a field, order by an instant, link by an edge). cortex computes every
value from the tool results this answer's turn received and renders the figure
(story:figure-svg-renderer). The model never types a value into a figure and never writes SVG: a
request that names a result the turn did not see, or a field it does not hold, is answered with a
tool error, and nothing is drawn.

## Why

A figure is evidence only if its numbers come from the store. Values the model types are values
nobody can trace; values counted from the turn's own tool results carry the same node and
evidence ids the answer cites.

## Domain relations

`cortex.ask.FigureRequest` (kind, title, source result references, grouping) in
`spec/domains/ask.yaml`, and the error `cortex.ask.FigureRefused`. A figure keeps the node and
evidence ids it was counted from. Under a redaction policy (story:ask-redaction) a figure is
counted from the redacted results the model saw, and its labels are restored like the answer.

## Work

- Spec first: the request type, the error, validate, `task generate`, conformance.
- The tool in the ask tool loop beside the reader's tools; the counting from the turn's results.
- The service page: the tool, what it counts, its refusals.

## Acceptance

- With a stand-in model that calls `figure` over two search results, the figure's bar counts equal
  the counts in those results, and its ids are the results' ids.
- A request naming a result id the turn did not receive is refused; a request with a literal
  value in place of a reference does not validate.
- Under a redaction policy no figure label holds an original value of a refused class.

---
format: aep.planning-md/3
id: story:ask-figure-event
kind: story
status: draft
title: Each figure reaches the client as a figure event and can be fetched again by answer and sequence
summary: A figure SSE event whose SVG equals a GET of the stored figure spec; the spec goes with its answer record
relations:
- decomposes: epic:instance-answers-questions
- depends_on: story:ask-figure-tool
- depends_on: story:answer-records-and-review
revision: 2
---
## Outcome

Each figure an answer draws reaches the client as a `figure` SSE event (`sequence`, `kind`,
`title`, `svg`, `nodes`, `evidence`), in order with the answer's deltas. The figure's spec is kept
beside the answer's record (story:answer-records-and-review), and `GET /ask/{answer_id}/figure/
{sequence}` renders it again: its body equals the event's `svg` byte for byte. The spec is
deleted with its answer record, by retention or otherwise.

## Why

A client that missed the stream, or a report, needs the figure again; rendering from the stored
spec rather than storing SVG keeps one renderer as the only source of figure markup.

## Domain relations

The event `cortex.ask.AskFigure` joins `cortex.ask.AskEvent`; `cortex.ask.FigureRecord`
(identity `answer_id` and `sequence`, the `FigureSpec`) is owned by `cortex.ask.AnswerRecord` and
goes with it.

## Work

- Spec first: the event, the record, the GET; validate, `task generate`, conformance.
- The records store keeps figure specs (schema version and migration), the GET endpoint under the
  same identity rule as the answer it belongs to.
- The event table and an example on the service page.

## Acceptance

- After an answer that draws two figures, the stream holds two `figure` events and each GET body
  equals its event's `svg`.
- After the answer record is purged, the GET answers 404 and no figure row is left.
- A GET by another identity than the asker's is refused as reading another answer is.

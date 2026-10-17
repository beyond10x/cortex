---
format: aep.planning-md/3
id: story:ask-report-download
kind: story
status: draft
title: An answer downloads as a report with its figures and its sources
summary: Reports in md, self-contained html with a script-free CSP, and optional pdf, rendered on request from stored specs
relations:
- decomposes: epic:instance-answers-questions
- depends_on: story:ask-figure-event
revision: 2
---
## Outcome

A reader downloads an answer as a report, rendered on request from the stored answer and figure
specs: `.md` with the figures as files beside it or inline, `.html` self-contained (figures
inline, a meta Content-Security-Policy that allows no script, no script element), and `.pdf`
where a renderer is available. Every report ends with a Sources section linking each node and
evidence id the answer and its figures cite, through the instance's link templates. A later
release than the figure tool.

## Why

A downstream deployment hands answers on as documents; a report rendered from the stored specs
needs no second copy of the content and goes when the record goes.

## Domain relations

`GET /ask/{answer_id}/report?format=md|html|pdf`; no new stored entity: a report is derived from
`cortex.ask.AnswerRecord` and its `cortex.ask.FigureRecord`s.

## Work

- Spec first: the endpoint and its formats; validate, `task generate`, conformance.
- The renderers for `.md` and `.html`; `.pdf` optional, stated on the service page if absent.

## Acceptance

- The `.html` report holds no `script` element and no event-handler attribute, and its CSP meta
  allows no script; it opens with no network request.
- The Sources section lists every cited node and evidence id once, each a link.
- After the answer record is purged, the report answers 404.

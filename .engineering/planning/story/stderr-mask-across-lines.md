---
format: aep.planning-md/3
id: story:stderr-mask-across-lines
kind: story
status: draft
title: A multi-line credential in a child's stderr is masked whole
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

A credential that spans several stderr lines (a PEM private key) is masked whole wherever
cortex reads a child's stderr line by line.

## Why

Adversary pass 2 on unit tail (`review-result:adversary-w30a-tail-pass-2`, finding 4): the
answer-records postgres launch's stderr (`src/ask/records/postgres.rs`) is masked one line at a
time, so a multi-line key's body lines would reach the ask service's log unmasked. Finding 3 of
the same pass showed the session stderr hold had the same class.

## Work

1. List every place that masks stderr per line.
2. Mask a PEM block across lines: from a BEGIN line to its END line, or to the end of the stream.

## Acceptance

- A test writes a PEM private key to the launch's stderr and finds none of its body lines in the
  ask service's log.

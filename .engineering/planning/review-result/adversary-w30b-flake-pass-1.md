---
format: aep.planning-md/3
id: review-result:adversary-w30b-flake-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the body-limit fix
relations:
- reviews: story:ask-body-limit-test-race
revision: 1
---
unit: flake (wave 20261030b), unit/w30b-flake at b6be5721, adversary pass 1 (opus), cases in tests/w30b_ask_adversary.rs (710fc577)
verdict: NEEDS-CHANGE
needs-coordinator: none

13 cases, 10 green, 3 red; origins from the same file run against base 7a94960c.

- Red, introduced (:650): the proxy drains a declared body before 401/413, so an
  `Expect: 100-continue` client gets its refusal only after the 30 s read timeout (base: at once);
  200 unauthenticated connections declaring 16 MiB and sending nothing stay held after 2 s.
- Red, pre-existing (:500): `POST /ask/` (404), `GET /ask` (405) and `GET /review` answer with a
  4 MiB body unread; the client gets a broken pipe. The new website line says "on every route".
- Red, pre-existing (:616): the proxy drains only a declared content-length; a chunked body's
  refusal is lost.
- Green: 1 MiB / 1 MiB+1 and 64 KiB / 64 KiB+1 edges; 16 MiB drain cap; 401 on all body routes;
  a stalled sender gets its 413 after 10.00 s; an endless chunked body gets its 413 7 ms after the
  first read; with 200 slow senders in the drain a legitimate /ask is answered in 56 ms (base 47 ms).

```findings
- file: src/ask/proxy.rs
  category: spec-gap
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the proxy drains a declared body before 401/413, holding an expect-continue client or a silent unauthenticated connection until the 30 s read timeout
- file: src/ask/serve.rs
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 404, 405 and GET /review answer with the body unread, contrary to the website line "on every route"
- file: src/ask/proxy.rs
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the proxy drains only a declared content-length, so a chunked body's refusal is lost
```

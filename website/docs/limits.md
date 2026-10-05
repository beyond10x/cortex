---
title: Limits
sidebar_position: 6
description: What cortex does not do today, and why.
---

# Limits

cortex is early. These are the limits that follow from the code and the specification as they
stand; [Status](./status.mdx) lists what is planned.

## Platform

- **Linux with systemd user units only.** Timers and the viewer are systemd user units; there is
  no other scheduler. `cortex create --no-units` creates an instance without them, and its sources
  then run only when you call `cortex run`.
- **One EKR version per instance**, pinned in the spec file. `cortex setup` installs EKR with
  `cargo install`, so it needs a Rust toolchain.

## Data

- **Credentials are masked; personal data is kept from the model, not from the store.** Fetched
  text is masked for credential shapes on every run: private keys, cloud, forge, chat and API
  token shapes, JSON Web Tokens and `password=`-style assignments. Masking is irreversible, so a
  credential in a document's text is never stored. A document's title, description and URL are
  not masked yet (`story:credential-mask-covers-titles`). An instance's `redaction` policy then
  replaces email addresses (also percent-encoded in a URL), phone numbers, IP addresses and
  payment card numbers (the classes `Email`, `Phone`, `IpAddress` and `PaymentCard`), and
  whatever its own regular-expression `rules` match, with placeholders such as `[Email-1]` in what
  the model is shown. That covers a document's text, title, description and
  source line, and the known entity names. Before the answer is applied, each placeholder is put
  back. The store and its evidence therefore hold the personal data and stay searchable for it;
  only the model does not see it. A placeholder the model changed or invented stays as written
  and is counted as `unrestored` in the run's log. Detection is by pattern: names of people are
  not detected, `Url`, `Credential` and `RareName` are not acted on yet, and a spec without a
  `redaction` policy sends text unchanged. Neither masking nor redaction is a guarantee.
- **A rule with a `replacement` is irreversible.** A `rules` entry whose `replacement` is not
  empty replaces what it matches by that text before the document is stored, so the store never
  holds it and nothing is put back from the model's answer. It covers a document's text, title and
  description; in a URL the match is hidden from the model but stored. A rule with
  `replacement: ""` uses a reversible placeholder (`[<rule name>-1]`) instead.
- **Some numbers that are not personal data are hidden from the model, but kept in the store.**
  Four-part version numbers and long digit ids that look like an IP address or a card number are
  replaced by a placeholder in the prompt.
- **A web page is cited as a statement, not as a URL.** EKR 0.0.30 admits only `HumanStatement`
  evidence in an extraction document, so cortex files a page as a `HumanStatement` whose identity is
  its URL. URL evidence waits on an EKR release that admits it.
- **Change detection is a content hash.** `ContentHash` is the only `change` value. A document
  whose text changed is extracted again only after `refresh_after_days`; cortex retracts nothing
  it applied from the earlier text.
- **Text only.** A files source skips files that are not UTF-8 text; web sources read the text the
  websearch adapter returns.

## Instances

- **The seed is fixed.** An instance's seed cannot change after creation; create a new instance
  instead.
- **`cortex update` only adds sources.** A source dropped from the spec keeps its registry entry
  and its timer.
- **`cortex remove` keeps the data.** The instance directory and its store stay on disk, and the
  name stays taken in the registry.

## The model

- **Extraction uses `claude -p`.** There is no other model runner. The model needs a signed-in
  `claude`; an `ANTHROPIC_API_KEY` in the environment is ignored.
- **The budget is per run.** `budget_usd` limits one run of one source, not a day or an instance.

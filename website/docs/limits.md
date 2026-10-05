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

- **No PII redaction.** Fetched text is masked for credential shapes only: private keys, cloud,
  forge, chat and API token shapes, JSON Web Tokens and `password=`-style assignments. Masking is
  not a guarantee. Do not point an instance at personal data.
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

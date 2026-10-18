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
- **One released target.** A release carries a binary for `x86_64-unknown-linux-gnu` only; on any
  other machine, build cortex from source.
- **One EKR version per instance**, pinned in the spec file. `cortex setup` installs EKR with
  `cargo install`, so it needs a Rust toolchain.

## Data

- **Credentials are masked; personal data is kept from the model, not from the store.** Fetched
  text is masked for credential shapes on every run: private keys, cloud, forge, chat and API
  token shapes, JSON Web Tokens, the password in a URL such as `https://user:password@host`, and
  `password=`-style assignments, also with a prefix (`DB_PASSWORD=`, `client_secret=`) or
  percent-encoded (`access_token%3D`). That covers a document's text, title, description and URL,
  and a URL or record key wherever cortex writes it: the run's `stopped` and `skipped` messages,
  its log and its state file. Masking is irreversible, so a credential in a listed shape is never
  stored. A URL that held one is stored and cited in its masked form, with the rest of its query
  kept, and two URLs that differ only in a credential are one document. Masking is by shape: a
  token of no listed shape with no name before it, or an assigned value shorter than six
  characters, is not masked. An instance's `redaction` policy then
  replaces links, email addresses (also percent-encoded in a URL), phone numbers, IP addresses
  and payment card numbers (the classes `Url`, `Email`, `Phone`, `IpAddress` and `PaymentCard`),
  the names it lists in `known_names`, rare capitalised words (`RareName`), and whatever its own
  regular-expression `rules` match, with placeholders such as `[Email-1]`, `[Url-1]` and
  `[Name-1]` in what the model is shown. That covers a document's text, title, description and
  source line, the store's ontology names and the known entity names; the question and every
  tool result of the [ask service](./ask-service.md#redaction) and of the eval `mcp` runner. Before the answer is
  applied or returned, each placeholder is put
  back. The store and its evidence therefore hold the personal data and stay searchable for it;
  only the model does not see it. A placeholder the model changed or invented stays as written
  and is counted as `unrestored` in the run's log. Detection is by pattern: a person's name is
  found only when `known_names` lists it or it is rare in the batch, and a spec without a
  `redaction` policy sends text unchanged. Neither masking nor redaction is a guarantee.
- **Links are found by their shape.** `Url` takes a link that starts with a scheme
  (`https://`), with `www.`, or with a dotted host followed by a path (`intranet.example.org/a`).
  A masked password or token inside a link stays part of it, so the whole link is one
  placeholder. A bare host with no path, such as `example.org`, is not taken.
- **Rare names are a guess.** `RareName` takes as a name a word that starts with a capital letter,
  has a lower-case letter in it, and appears at most `rare_limit` times (default 1) in the batch's
  document texts. Only text the model is shown counts: a word inside an address, a link, a known
  name or a credential another class hides is not counted, and neither are the known entity
  names. For a record of a `files` source read as records, only the record's own text counts,
  not its thread context, key or title, which repeat other records' text and the file's name. A
  known entity name the batch's documents do not hold counts as seen 0 times, so a
  person the store already knows is hidden in the prompt's known entities too. Common sentence
  openers (`The`, `This`, `We` and the like) are never taken. A rare product or place name, and
  a capitalised word such as `Thanks` or `Subject`, is hidden from the model too, and restored; a
  person named often in the batch and not listed in `known_names` is not hidden. A word with
  hyphens, such as `Jean-Luc` or `Co-Authored-By`, is one word: it is hidden whole or kept
  whole, and no part of a word seen more than `rare_limit` times is taken. A name glued directly
  to a lower-case letter (`useFoo-Bar`), or after underscores to a run of letters and digits that
  begins with a lower-case letter (`snake_case_Identifier`, `lebenslauf_Rowan`), is read as part
  of an identifier and not taken. Otherwise an underscore and CJK text bound a word as a space
  does, so `_Rowan_`, `CV_Rowan.pdf`, the `Quill` of `Rowan_Quill` and the `Rowan` of
  `会议上Rowan发言了` are taken.
- **Known names are matched in any case.** A name in `known_names` is matched as a whole word,
  in any upper or lower case, and in either Unicode form (composed `ë` or `e` with a combining
  mark).
- **`Credential` adds shapes and is irreversible.** The class masks, as `[masked:<shape>]`,
  shapes the masking on every run does not cover: bearer and basic `Authorization` headers, more
  cloud, forge, chat and model-provider token shapes, chat webhook URLs, and assignments to a
  wider set of names (`token`, `pwd`, `passphrase`, `private_key`, `credentials`,
  `authorization`; not `auth` alone) whose value mixes letters and digits or is at least 20
  characters long. In text an assigned value runs to whitespace or a quote, `;#&,<>` included;
  in a document's URL it ends at `&` or `#`, so the rest of the query is kept. A value that is a
  placeholder (`<your-key>`, `${TOKEN}`, `DEPLOY_TOKEN`, `xxxx`, `[masked:…]`) or a digest
  (`sha256:…`) is kept. Its matches are neither sent nor stored, in a document's URL either.
- **A run refuses what masking left.** The classes `refuse_if_left` names are looked for again in
  every batch as the model would be shown it. Every batch is checked before the first model
  call; a class still found there fails the run (`extraction-failed`, the reason naming the
  class), and nothing is sent or stored. Each batch is checked again just before it is sent,
  because the known entity names grow as batches are applied: a class found then stops the run
  before that batch is sent, and the batches before it stay applied. For `Phone` the search adds
  a number of 6 to 15 digits shortly after a word such as "call", "phone" or "mobile", which the
  `Phone` class itself does not replace. A class named in `refuse_if_left` but not in `classes`
  refuses every batch in which it is found.
- **A rule with a `replacement` is irreversible.** A `rules` entry whose `replacement` is not
  empty replaces what it matches by that text before the document is stored, so the store never
  holds it and nothing is put back from the model's answer, as for `Credential`. It covers a
  document's text, title and description; in a URL the match is hidden from the model but stored. A rule with
  `replacement: ""` uses a reversible placeholder (`[<rule name>-1]`) instead.
- **Some numbers that are not personal data are hidden from the model, but kept in the store.**
  Four-part version numbers and long digit ids that look like an IP address or a card number are
  replaced by a placeholder in the prompt.
- **A web page is cited as a statement, not as a URL.** EKR 0.0.32 admits only `HumanStatement`
  evidence in an extraction document (`extraction-evidence-kind-unsupported` for any other), so
  cortex files a page as a `HumanStatement` whose identity is its URL. URL evidence waits on an EKR
  release that admits it (EKR `story:extraction-admits-url-evidence`; here
  `upstream-blocker:ekr-url-evidence`).
- **Relations in a store extracted before EKR 0.0.31 are not graph edges.** Since EKR 0.0.31 an
  extracted relation is also written as an edge, so `search` counts it in a node's `degree` and
  `expand` walks it. EKR does not backfill: relations a store took under EKR 0.0.30 or earlier stay
  assertions without an edge, and graph reads do not walk them.
- **Change detection is a content hash.** `ContentHash` is the only `change` value. A document
  whose text changed is extracted again only after `refresh_after_days`; cortex retracts nothing
  it applied from the earlier text. A record read from a file, and a `structured` change whose
  links to tags alone changed, are applied again on the next run.
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

- **Extraction uses `claude -p`.** There is no other working model runner. The model needs a
  signed-in `claude`, or a `model.gateway` in the spec, whose token is read from the variable it
  names; an `ANTHROPIC_API_KEY` in the environment is ignored either way. The spec accepts
  `model.backend: Codex`, but that backend extracts nothing in this release: every run, quality
  measurement and schema round of such an instance fails, naming `story:codex-model-backend`.
- **A model outage is told by its error text.** A run ends `model-unavailable`, not counted
  against the source, only when the model call's error names a refused connection, a model not
  loaded, or HTTP 502, 503, 504 or 529 (overloaded) in words cortex knows (see
  [When the model cannot be reached](./operating.md#when-the-model-cannot-be-reached)); an outage
  a gateway words otherwise, or a call that times out, ends `extraction-failed` and is counted.
  The run does not retry the model beyond the one repeat of an error that cost zero.
- **A run's figures count only what was reported after a batch was applied.** `metrics.prom` adds
  the documents and model tokens of a run that ended `ran` or applied a batch before it ended
  otherwise; a run that ended before it applied a batch adds none of them, though a failed model
  call may have spent tokens, and a model call ended by SIGTERM never reports its tokens.
- **A stopped fetch keeps nothing.** A run sent SIGTERM during its fetch ends the Connectors call
  in flight and drops the pages it already read: the next run reads the whole window again.
- **The daily ceiling is per instance.** `daily_ceiling_usd` bounds what one instance's model calls
  spend in a UTC day; nothing bounds the instances of a home together. An answer that states no
  cost adds nothing to the spend of the day.
- **The dollar limits do not bound a model `claude` cannot price.** An answer's cost is the
  `total_cost_usd` `claude` states, worked out from its own price table. For a model id that table
  does not know, what `claude` reports is unverified: no `total_cost_usd` at all, or a figure that
  is not what the model costs. An answer with no cost spends none of `budget_usd` and adds nothing
  to `daily_ceiling_usd`, so neither stops such a run. Bound it in tokens instead, with
  `budget_tokens` and `daily_ceiling_tokens` (see
  [the spec file](./spec-file.md#limits-in-tokens)).
- **Behind a model gateway, only tokens bound a run.** `claude` prices a gateway's answer at its
  default model's rate, so cortex counts that cost against no dollar limit: `budget_tokens` is
  required, `budget_usd` and `daily_ceiling_usd` bound nothing, and the reported cost is recorded
  as not counted (`cost_counted: false`, `uncounted_usd` in `spend.json`). cortex does not price a
  gateway model's tokens in dollars (see [A model gateway](./spec-file.md#a-model-gateway)). A
  gateway instance created by an earlier release with no `budget_tokens` keeps running, and its
  dollar limits still count the reported cost, as before, until `cortex update` gives it a
  `budget_tokens`.
- **The batch size is an estimate, not a token count.** Without `model.batch_chars`, a batch
  behind a gateway is sized at 1.5 characters per token of the model's window; dense text such as
  code can still be too long, or draw an answer cut off at the output limit. cortex then splits
  the batch, at most 3 deep; a part of several documents that still does not fit ends the run as
  `batch-too-large`, and one document that alone does not fit is left out, named in `too_large`
  and not extracted until its text changes (see
  [The batch size](./spec-file.md#the-batch-size)). The size counts UTF-8 bytes, so text in a
  non-Latin script fills a batch 2 to 3 times as fast. `cortex quality` and `cortex schema`
  keep their own batches of 20 facts.
- **A token limit is checked between calls, not within one.** `claude` takes no token limit, so a
  run, quality measurement or schema round can pass `budget_tokens` or `daily_ceiling_tokens` by
  the tokens of its last call, and commands running at once can each pass the daily one by a
  call, as with the dollar ceiling.
- **Token counts are what the answer reports.** cortex trusts the counts in an answer's `usage`
  as the gateway or `claude` reports them. A count it cannot use (missing, below 0, or a sum past
  a signed 64-bit integer) is treated as uncounted, which under a token limit stops the run
  before its next call; it is never taken as zero.
- **Commands running at once can each overshoot the daily ceiling by one call.** A call's
  allowance is read from `spend.json` without the home's lock, and its cost is added only after
  the answer. A run, a `cortex quality` and a `cortex schema` of one instance can run at the same
  time (the last two let the lock go during their model calls), so each may be given what the
  ceiling left before the others' answers were added.

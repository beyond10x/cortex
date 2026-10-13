---
format: aep.planning-md/3
id: story:unattended-model-credential-design
kind: story
status: draft
title: The unattended model credential path is designed
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A written design of how extraction gets a model credential with nobody signed in: which credential
`claude -p` uses in a container, where it is held so that cortex never holds it, what changes in the
isolated call, and how spend stays bounded. The design is the deliverable; entering a key and any
spend it carries are the operator decision, not this story.

## Why

Verified in this tree:

- Extraction is one isolated `claude -p` call that removes `ANTHROPIC_API_KEY` from its environment
  so the interactive sign-in is used (`src/extract.rs:255-256`, `:282`; AGENTS.md "The model call is
  isolated", `AGENTS.md:153-157`, which also says "Keep the flags").
- "The model needs a signed-in `claude`; an `ANTHROPIC_API_KEY` in the environment is ignored"
  (`website/docs/limits.md:117-118`).
- The call passes `--setting-sources ""` (`src/extract.rs:271`), so no settings file reaches it.

Not confirmed (read from `claude --help` of `claude` 2.1.296 on the drafting machine, nothing tried):
the CLI lists `setup-token` ("Set up a long-lived authentication token") and mentions an
`apiKeyHelper` given through `--settings`. Whether either works unattended in a container, and under
which terms, is for the design to establish.

## Work

- Write the design as a `design` artifact in this store that `designs` this story. It answers:
  the credential kind and who issues it; where it is held (Connectors custody, a mounted secret read
  only by the `claude` process, or another place) so that it never enters cortex, its spec file, its
  logs or its run directories; which of the isolation flags and the `ANTHROPIC_API_KEY` removal
  change, and why that keeps the measured isolation (AGENTS.md:153-157); how a missing or expired
  credential fails a run and is reported; how spend is bounded per run (`--max-budget-usd`, already
  passed, `src/extract.rs:280`) and per day (`story:daily-instance-budget`).
- Spec first: the design names every noun, field, command or outcome it would add, as a change to
  `spec/` that the implementing story makes first. This story changes no code and no spec.
- The design lists the steps the operator takes (obtaining and entering the credential) as the
  operator steps, outside any story.

## Acceptance

- A `design` artifact that `designs` this story is added to the store, answering each question in
  Work with a citation or a labelled open question.

## Not in this story

Choosing or paying for a credential; entering one; implementing the design (a later story, drafted
from the design); the Codex backend (`story:codex-model-backend`).

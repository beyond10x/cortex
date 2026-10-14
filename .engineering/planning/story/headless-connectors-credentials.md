---
format: aep.planning-md/3
id: story:headless-connectors-credentials
kind: story
status: draft
title: A run in a container reaches its connections without a desktop Secret Service
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- depends_on: story:container-image
revision: 1
---
## Outcome

A cortex run in a container with no desktop session reads its sources and opens its PostgreSQL
store through Connectors connections, using the headless credential custody Connectors provides.

## Why

Verified in this tree (cortex side):

- cortex reaches every source credential only through the `connectors` CLI: `connections list`
  (`src/connectors.rs:102-105`), `connections revalidate` when a connection is not `ready`
  (`:122-148`) and `operations invoke` for each read (`:270-275`). A PostgreSQL store with a
  connection starts every `ekr` through `connectors connections launch` (`:173`, `:200`). cortex
  holds no credential itself (AGENTS.md "No secret enters cortex").

Read, not run (inference; not confirmed in a container):

- The Connectors custody binding accepts one qualified Secret Service daemon (`gnome-keyring`, by
  binary digest) on an ext-family filesystem (Connectors repository,
  `docs/local-secret-service.md:13-20`); it authenticates the session bus at `/run/user/<uid>/bus`
  by default (`:22-25`) and has "no service activation, unlock, or prompt" (`:30`); only the
  `login` collection, already unlocked, is selected (`:32-33`). A container has no desktop session
  that unlocks a `login` collection, so the reads cortex makes would fail there. This is read from
  the document; no container run was made to confirm it.

Headless custody is a Connectors change. This story is its consumer: its acceptance waits on a
Connectors release that provides headless credential custody (an upstream blocker records it).

## Work

- Once the Connectors capability is released: configure the container so `connectors` uses the
  headless custody, with no credential passing through cortex, its spec file, its logs or its
  environment as cortex writes it.
- Pin the Connectors version that provides it where cortex pins its tools, and document the
  container setup on the operating page (its PostgreSQL password section and the source sections).
- Spec first: if cortex needs a new setting to select the custody (it should not: custody is
  Connectors configuration), it goes into `spec/` first; otherwise no spec change.
- An end-to-end case with a real `connectors` in a container, or the closest stand-in the tests
  allow, reads one source and opens one PostgreSQL store.

## Acceptance

- In a container with no session bus, a `cortex run` of a connectors source moves from start to the
  `ran` outcome, its connection read through the headless custody.
- In the same container, a run on a `postgres` store opened through `connectors connections launch`
  ends `ran` with the store head advanced.

## Not in this story

The Connectors custody itself (upstream); the model credential
(`story:unattended-model-credential-design`); the container image (`story:container-image`).

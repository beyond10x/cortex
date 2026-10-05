---
slug: /
title: Overview
sidebar_label: Overview
sidebar_position: 1
description: What cortex is, what it owns, and what it leaves to EKR, Connectors and the model.
---

# cortex

cortex spins up knowledge instances on the
[Epistemic Knowledge Runtime](https://github.com/beyond10x/epistemic-knowledge-runtime) (EKR).
One spec file defines an instance: its seed schema, its model settings and its data sources.
Each instance is one EKR store in its own SQLite file. systemd user timers run every source on its
own schedule, a model extracts what new or changed documents say, and EKR applies the result to
the store. You browse a store with `ekr view` and serve it to agents over MCP with `ekr mcp`.

```text
spec file          name, seed schema, model, sources, schedules
   ↓  cortex create
instance           one directory, one EKR SQLite store, one viewer, one timer per source
   ↓  timer fires: cortex run <instance>/<source>
fetch              web pages or operation records through Connectors, or local files
   ↓
keep               only documents that are new or whose text changed
   ↓
extract            one isolated model call per batch → an EKR extraction document
   ↓
apply              ekr apply-extraction into the instance's store
   ↓
serve              ekr view in a browser, ekr mcp to agents
```

## Who owns what

| part | owner |
|---|---|
| instances, sources, schedules, runs, what a source has already seen | cortex |
| the store, its schema and its validation; the viewer and the MCP server | EKR (`ekr` binary) |
| every integration and every credential | [Connectors](https://github.com/beyond10x/connectors) (`connectors` binary) |
| reading the text and answering in EKR's extraction format | the model, through `claude -p` with no tools |

cortex holds no secrets. A source names a Connectors connection id, and cortex only invokes
operations through `connectors`; the credential stays in Connectors.

## Where to go next

- [Quickstart](./quickstart.md): from nothing to a running instance.
- [Spec file](./spec-file.md): every section of a `cortex.instance/1` file.
- [Commands](./commands.md): what each `cortex` command does and prints.
- [Operating an instance](./operating.md): where things live, what a run does, failures.
- [Limits](./limits.md): what cortex does not do today.
- [Status](./status.mdx): shipped and planned, item by item.
- [ESS specification](./reference/ess/index.mdx): the specification cortex is generated from.

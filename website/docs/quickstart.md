---
title: Quickstart
sidebar_position: 2
description: Build cortex, connect a web search provider, create an instance and run a source.
---

# Quickstart

## Requirements

- Linux with systemd user units. Timers and the viewer are systemd user units.
- Rust and Cargo, to build cortex and to let `cortex setup` install EKR.
- `ekr` at the version the spec file pins. `cortex setup` installs it.
- [`connectors`](https://github.com/beyond10x/connectors), with a connection for every adapter
  your sources use.
- `claude`, signed in. cortex removes `ANTHROPIC_API_KEY` from the model call's environment, so the
  sign-in is what it uses.

## 1. Build and install

```sh
git clone https://github.com/beyond10x/cortex.git
cd cortex
cargo install --locked --path .
cortex setup
```

`cortex setup` installs the pinned `ekr` (0.0.31 unless you pass `--ekr-version`) with
`cargo install` under `~/.cache/cortex/bin/<version>/`, and does nothing when it is already there.

## 2. Connect a web search provider

Web sources read websites through a Connectors adapter that implements
`datasource.websearch/v1alpha1`; `tavily` is the default. Connect it once and note the connection
id it lists:

```sh
connectors connections connect --adapter tavily --profile <the provider's API-key profile> --credential-prompt
connectors connections list --adapter tavily
```

## 3. Write the spec file

Copy `examples/` from the repository and edit `example.yaml`: the name, the description, the
connection id, and your queries or sites. `examples/seed/schema.yaml` is the starting ontology, in
EKR's own `ekr.extraction-document/1` format. [Spec file](./spec-file.md) explains every section.

`cortex schema` prints the spec file's JSON Schema, for an editor that validates YAML against one.

## 4. Create the instance

```sh
cortex create --spec examples/example.yaml
```

This checks that Connectors lists every connection the sources name, seeds the store, applies the
seed schema, starts the viewer and installs one timer per source. It prints one JSON line with
the instance directory, the viewer's address and the source ids.

## 5. Run a source now

Instead of waiting for its timer:

```sh
cortex run example/news
cortex list
cortex sources
```

`cortex list` shows the viewer's address (`http://127.0.0.1:<port>/`). To give an agent the store,
run the line `cortex mcp-line example` prints:

```sh
cortex mcp-line example
```

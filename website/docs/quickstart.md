---
title: Quickstart
sidebar_position: 2
description: Install cortex from a release, connect a web search provider, create an instance and run a source.
---

# Quickstart

## Requirements

- Linux on x86-64 with systemd user units. Timers and the viewer are systemd user units, and the
  released binary is built for `x86_64-unknown-linux-gnu`.
- Rust and Cargo, so that `cortex setup` can install EKR (and to build cortex from source).
- `ekr` at the version the spec file pins. `cortex setup` installs it.
- [`connectors`](https://github.com/beyond10x/connectors), with a connection for every adapter
  your sources use.
- `claude`, signed in. cortex removes `ANTHROPIC_API_KEY` from the model call's environment, so the
  sign-in is what it uses.

## 1. Install

Each [GitHub Release](https://github.com/beyond10x/cortex/releases) carries two files:
`cortex-<version>-x86_64-unknown-linux-gnu.tar.gz`, which holds the `cortex` binary, `LICENSE`
and `README.md` in a directory of the same name, and `SHA256SUMS`, the tarball's checksum.
Download both, check the tarball against the checksum, and put the binary on your `PATH`:

```sh
version=0.2.2
base=https://github.com/beyond10x/cortex/releases/download/v$version
curl -fsSLO "$base/cortex-$version-x86_64-unknown-linux-gnu.tar.gz"
curl -fsSLO "$base/SHA256SUMS"
sha256sum --check --strict SHA256SUMS
tar -xzf "cortex-$version-x86_64-unknown-linux-gnu.tar.gz"
install -m 755 "cortex-$version-x86_64-unknown-linux-gnu/cortex" ~/.local/bin/cortex
cortex --version
cortex setup
```

`sha256sum` prints `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz: OK`; any other answer means
the download is not the released file, and it must not be installed. `cortex --version` prints
`cortex <version>`. With the [GitHub CLI](https://cli.github.com/),
`gh release download v$version --repo beyond10x/cortex` downloads both files in one step.

`cortex setup` installs the pinned `ekr` (0.0.32 unless you pass `--ekr-version`) with
`cargo install` under `~/.cache/cortex/bin/<version>/`, and does nothing when it is already there.

### Building from source

To run a commit that is not released, build it with Cargo instead:

```sh
git clone https://github.com/beyond10x/cortex.git
cd cortex
cargo install --locked --path .
cortex setup
```

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

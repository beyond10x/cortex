# cortex

Spin up knowledge brains from a spec. Each instance is one [EKR](https://github.com/beyond10x/epistemic-knowledge-runtime)
store in its own SQLite file, fed on a schedule by the data sources the spec connects:

| source kind | what it fetches |
|---|---|
| `web` | websites with a title, a description and their content, through Tavily: either **search** (a list of queries) or **sites** (a list of URLs, read as pages or crawled), each with its own policy |
| `connectors` | the records of any operation a [Connectors](https://github.com/beyond10x/connectors) adapter admits |
| `files` | local files matching a glob |

A run keeps only new or changed documents, has a model extract what they say into an EKR
extraction document, and applies it to the store. Every fact cites the document it came from.
`ekr view` shows the store in a browser and `ekr mcp` serves it to agents.

cortex holds no secrets. A source names a Connectors connection; you set and manage the
credential in Connectors.

## Requirements

- `ekr` at the version the spec pins (`cortex setup` installs it with `cargo install`)
- `connectors`, with a connection for every adapter your sources use
- `claude`, signed in
- systemd user units (Linux)

## Quickstart

1. Connect Tavily once, and note the connection id it prints:

   ```sh
   connectors connections connect --adapter tavily --profile <the provider's API-key profile> --credential-prompt
   connectors connections list --adapter tavily
   ```

2. Copy `examples/` and edit `example.yaml`: name, description, the connection id, your queries
   or sites. `examples/seed/schema.yaml` is the starting ontology, in EKR's own
   `ekr.extraction-document/1` format.

3. Create the instance. It seeds the store, applies the schema, starts the viewer and installs one
   timer per source:

   ```sh
   cortex create --spec examples/example.yaml
   ```

4. Run a source now instead of waiting for its timer, and look at the result:

   ```sh
   cortex run example/news
   cortex list
   cortex sources
   ```

`cortex schema` prints the JSON Schema of the spec file. The specification of cortex itself is
`spec/` (ESS); `generated/docs/` is its rendered form.

## Commands

| command | does |
|---|---|
| `cortex create --spec <file>` | create an instance and add its sources |
| `cortex update <name> --spec <file>` | change sources, model or serve settings; a seed change is refused |
| `cortex run <instance>/<source>` | run one source once |
| `cortex remove <name>` | remove the timers and the viewer; the directory and store stay |
| `cortex source enable <instance>/<source>` | re-enable a source disabled after two failures in a row |
| `cortex list`, `cortex sources` | instances and sources with their state |
| `cortex mcp-line <name>` | print the `claude mcp add` line for the instance's store |
| `cortex setup` | install the pinned `ekr` if it is missing |
| `cortex schema` | print the spec file's JSON Schema |

Agents changing this repository read [AGENTS.md](AGENTS.md).

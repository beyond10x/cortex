# cortex

Spin up knowledge instances from one spec file. Each instance is one
[EKR](https://github.com/beyond10x/epistemic-knowledge-runtime) store, in its own SQLite file or in PostgreSQL, fed
on a schedule by the data sources the spec connects:

| source kind | what it fetches |
|---|---|
| `web` | websites with a title, a description and their content, through any Connectors adapter implementing `datasource.websearch/v1alpha1` (`tavily` by default): either **search** (a list of queries) or **sites** (a list of URLs, read as pages or crawled), each with its own policy |
| `connectors` | the records of any operation a [Connectors](https://github.com/beyond10x/connectors) adapter admits |
| `files` | local files matching a glob |

A run keeps only new or changed documents, has a model extract what they say into an EKR
extraction document, and applies it to the store. Every fact cites the document it came from.
`ekr view` shows the store in a browser and `ekr mcp` serves it to agents.

cortex holds no secrets. A source names a Connectors connection; you set and manage the
credential in Connectors.

**Documentation: <https://beyond10x.github.io/cortex/>** — the spec file section by section,
every command, operating notes, limits and the generated specification reference.

## Requirements

- Linux with systemd user units
- Rust and Cargo
- `ekr` at the version the spec pins (`cortex setup` installs it with `cargo install`)
- `connectors`, with a connection for every adapter your sources use
- `claude`, signed in

## Quickstart

1. Build and install cortex, then the pinned EKR:

   ```sh
   cargo install --locked --path .
   cortex setup
   ```

2. Connect Tavily once, and note the connection id it prints:

   ```sh
   connectors connections connect --adapter tavily --profile <the provider's API-key profile> --credential-prompt
   connectors connections list --adapter tavily
   ```

3. Copy `examples/` and edit `example.yaml`: name, description, the connection id, your queries
   or sites. `examples/seed/schema.yaml` is the starting ontology, in EKR's own
   `ekr.extraction-document/1` format.

4. Create the instance. It seeds the store, applies the schema, starts the viewer and installs one
   timer per source:

   ```sh
   cortex create --spec examples/example.yaml
   ```

   The timers run with the `PATH`, `HOME` and `connectors`/`claude` binaries of the shell you run
   this from. For runs while you are logged out, enable lingering once: `loginctl enable-linger`.

5. Run a source now instead of waiting for its timer, and look at the result:

   ```sh
   cortex run example/news
   cortex list
   cortex sources
   ```

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

Options, outcomes and exit statuses are on the [Commands](https://beyond10x.github.io/cortex/docs/commands)
page.

## Building from source

```sh
task check
```

runs the specification checks, formatting, clippy, every test and the documentation drift check.
It needs the [Task runner](https://taskfile.dev/), the `ess` CLI and an `ekr` binary for the
end-to-end tests. `task website` builds the documentation site under `website/`.

The specification of cortex itself is `spec/` (ESS); `generated/docs/` is its rendered form.

## Contributing

Commands, gates, conventions and the rules this repository holds are in [AGENTS.md](AGENTS.md).

## License

Apache-2.0. See [LICENSE](LICENSE).

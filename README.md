# cortex

Spin up knowledge instances from one spec file. Each instance is one
[EKR](https://github.com/beyond10x/epistemic-knowledge-runtime) store, in its own SQLite file or in PostgreSQL, fed
on a schedule by the data sources the spec connects:

| source kind | what it fetches |
|---|---|
| `web` | websites with a title, a description and their content, through any Connectors adapter implementing `datasource.websearch/v1alpha1` (`tavily` by default): either **search** (a list of queries) or **sites** (a list of URLs, read as pages or crawled), each with its own policy |
| `connectors` | the records of any operation a [Connectors](https://github.com/beyond10x/connectors) adapter admits |
| `files` | local files matching a glob |
| `structured` | records from a Connectors operation or from JSON files, mapped to nodes, properties and relations by a fixed mapping, with no model call |

A run keeps only new or changed documents, has a model extract what they say into an EKR
extraction document, and applies it to the store. Every fact cites the document it came from.
`ekr view` shows the store in a browser and `ekr mcp` serves it to agents.

What an instance does besides:

- **Redaction.** A spec's `redaction` policy replaces email addresses, phone numbers, links, names
  and the patterns it lists with placeholders in what the model is shown, and puts the values back
  before anything is stored. Credential shapes are masked on every run and never stored.
- **Snapshots and undo.** Before a run applies anything to a SQLite store, cortex snapshots the
  store and the source state; `cortex restore` puts both back. A run that fails the spec's `gate`
  checks is undone from its snapshot.
- **Adoption.** `cortex adopt` makes an existing EKR store an instance, with its whole history,
  without reseeding it.
- **Quality and schema.** `cortex quality` has the model judge a sample of facts against their
  evidence; `cortex schema` has it propose ontology changes, and applies those EKR accepts.

cortex holds no secrets. A source names a Connectors connection; you set and manage the
credential in Connectors. A PostgreSQL store's password can come from a Connectors connection too.

**Documentation: <https://beyond10x.github.io/cortex/>** — the spec file section by section,
every command, operating notes, limits and the generated specification reference.

## Requirements

- Linux on x86-64 with systemd user units
- Rust and Cargo, for `cortex setup`
- `ekr` at the version the spec pins (`cortex setup` installs it with `cargo install`)
- `connectors`, with a connection for every adapter your sources use
- `claude`, signed in

## Quickstart

1. Download a release's tarball and `SHA256SUMS` from
   [Releases](https://github.com/beyond10x/cortex/releases), check the tarball, install the binary,
   then the pinned EKR:

   ```sh
   gh release download v0.2.2 --repo beyond10x/cortex
   sha256sum --check --strict SHA256SUMS
   tar -xzf cortex-0.2.2-x86_64-unknown-linux-gnu.tar.gz
   install -m 755 cortex-0.2.2-x86_64-unknown-linux-gnu/cortex ~/.local/bin/cortex
   cortex setup
   ```

   To build from a checkout instead: `cargo install --locked --path .`.

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
| `cortex adopt --spec <file> --store <file>` | make an existing EKR store an instance, with its history, without reseeding it |
| `cortex update <name> --spec <file>` | change sources, model or serve settings; a seed change is refused |
| `cortex run <instance>/<source>` | run one source once |
| `cortex restore <name> <snapshot>` | put a SQLite store and the source state back to a snapshot a run took |
| `cortex quality <name> --sample <n>` | measure how often facts are supported by the evidence they cite |
| `cortex schema <name>` | propose ontology changes from a sample of facts and apply those EKR accepts |
| `cortex remove <name>` | remove the timers and the viewer; the directory and store stay |
| `cortex source enable <instance>/<source>` | re-enable a source disabled after two failures in a row |
| `cortex list`, `cortex sources` | instances and sources with their state |
| `cortex mcp-line <name>` | print the `claude mcp add` line for the instance's store |
| `cortex setup` | install the pinned `ekr` if it is missing |
| `cortex schema` | without an instance, print the spec file's JSON Schema |

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

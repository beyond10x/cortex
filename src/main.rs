//! The `cortex` command line: clap derive over the generated behaviours of `spec/`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use clap::{Parser, Subcommand};
use cortex_model::behaviour::Generated;
use cortex_model::instance as m;
use cortex_model::ports::cortex::Cortex;
use cortex_model::primitives::Decimal;
use serde_json::{json, Value};

use cortex_cli::connectors::Connectors;
use cortex_cli::home::Home;
use cortex_cli::instance::{Layout, Meta};
use cortex_cli::ports::{Ports, Shared, SharedRef};
use cortex_cli::schedule::{self, Systemd};
use cortex_cli::snapshot::{self, Refusal, RestoreError};
use cortex_cli::{ekr, home, instance, model_map, quality, run, schema, spec};

/// Spin up EKR knowledge brains from a spec, fed on a schedule by Connectors data sources.
#[derive(Parser)]
#[command(name = "cortex", version)]
struct Cli {
    /// Where instances live.
    #[arg(long, env = "CORTEX_HOME", global = true)]
    home: Option<PathBuf>,
    /// The `connectors` binary.
    #[arg(
        long,
        env = "CORTEX_CONNECTORS",
        default_value = "connectors",
        global = true
    )]
    connectors: PathBuf,
    /// The `claude` binary.
    #[arg(long, env = "CORTEX_CLAUDE", default_value = "claude", global = true)]
    claude: PathBuf,
    /// The `codex` binary, for an instance whose spec names `model.backend: Codex`.
    #[arg(long, env = "CORTEX_CODEX", default_value = "codex", global = true)]
    codex: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an instance from a spec file: seed its store, add its sources, start its viewer and
    /// timers (`cortex.instance.CreateInstance`).
    Create {
        #[arg(long)]
        spec: PathBuf,
        /// Do not extract the seed documents now.
        #[arg(long)]
        no_extract: bool,
        /// Install no systemd units.
        #[arg(long)]
        no_units: bool,
        /// Replace `<home>/bin/cortex` even when it is a newer cortex than this one. Every timer of
        /// the home runs that binary; without this flag a newer one is kept.
        #[arg(long)]
        replace_binary: bool,
        /// For a `postgres` store: the `ekr.postgres/1` file of the schema-management role.
        /// `ekr postgres-schema` provisions the provider's tables with it before the seed. Without
        /// it the tables must already exist.
        #[arg(long)]
        postgres_schema_config: Option<PathBuf>,
    },
    /// Make an existing EKR store an instance, with its whole revision history, without seeding,
    /// extracting or writing to it (`cortex.instance.AdoptInstance`).
    ///
    /// A SQLite store is copied into the instance directory through SQLite's online backup, with
    /// the revisions still in its `-wal`, and the instance grows the copy. The database named is
    /// not written; SQLite may leave a `-shm` (and an empty `-wal`) beside it. The copy needs the
    /// store's size plus 64 MiB free under the home. A PostgreSQL store is used where it is:
    /// `--store` names the `ekr.postgres/1` file the spec's `store.value.config` names, and no
    /// other active instance may already grow that file's lineage under the same tenant.
    /// Every node and edge type of the spec's `seed.ekr_seed` and `seed.schema` must be in the
    /// store; the seed documents are not extracted.
    ///
    /// Each source starts from the `cortex.seen/1` document `--seen` names for it, or empty. An
    /// empty one has the first run fetch every document and apply each whose content hash it has
    /// not seen, which re-extracts the documents the store already holds: a second set of
    /// assertions and evidence for them, at the model's cost. A seen document is not extracted
    /// again even when the store holds no evidence of it; `adopted` counts those as
    /// `seen_without_evidence`.
    #[command(verbatim_doc_comment)]
    Adopt {
        #[arg(long)]
        spec: PathBuf,
        /// The SQLite database file, or for a `postgres` spec its `ekr.postgres/1` file.
        #[arg(long)]
        store: PathBuf,
        /// The `ekr.cli-host/1` document the store was seeded under, copied as the instance's
        /// host. EKR opens a store only under the tenant and authority it was seeded with. Without
        /// it, cortex writes its own host for tenant `<name>`, which opens a store cortex created.
        #[arg(long)]
        host: Option<PathBuf>,
        /// `<source>=<file>`: the seen documents the source starts from, a `cortex.seen/1`
        /// document. Repeat it for each source, once per source.
        #[arg(long, value_parser = seen_arg)]
        seen: Vec<(String, PathBuf)>,
        /// Install no systemd units.
        #[arg(long)]
        no_units: bool,
        /// Replace `<home>/bin/cortex` even when it is a newer cortex than this one. Every timer of
        /// the home runs that binary; without this flag a newer one is kept.
        #[arg(long)]
        replace_binary: bool,
    },
    /// Change an instance's sources, model or serve settings (`cortex.instance.UpdateInstance`).
    Update {
        name: String,
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        no_units: bool,
        /// Replace `<home>/bin/cortex` even when it is a newer cortex than this one. Every timer of
        /// the home runs that binary; without this flag a newer one is kept.
        #[arg(long)]
        replace_binary: bool,
    },
    /// Remove an instance's timers and viewer; its directory and store stay
    /// (`cortex.instance.RemoveInstance`).
    Remove { name: String },
    /// Run one source once (`cortex.instance.RunSource`).
    Run {
        /// `<instance>/<source>`.
        source_id: String,
        /// On a failed run, record the failure (`cortex.instance.RecordFailure`), as the timers do.
        #[arg(long)]
        record_failure: bool,
    },
    /// Put an instance's store back to a snapshot a run took before it applied anything
    /// (`cortex.instance.RestoreSnapshot`).
    Restore {
        name: String,
        /// A file name under `<instance>/snapshots/`, without `.sqlite`.
        snapshot: String,
    },
    /// Measure how often the instance's facts are supported by the evidence they cite: draw a
    /// sample, have the instance's model judge it in batches of 20 and write the pass rate with
    /// its Wilson interval under `quality/<UTC stamp>/` (`cortex.instance.MeasureQuality`).
    Quality {
        name: String,
        /// How many facts to draw and judge, 1 to 1000.
        #[arg(long)]
        sample: i64,
    },
    /// One data source of an instance.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Instances (`cortex.instance.Instances`).
    List,
    /// Sources (`cortex.instance.Sources`).
    Sources,
    /// Print the `claude mcp add` line that serves an instance's store to agents.
    McpLine { name: String },
    /// Install the pinned `ekr` when it is missing.
    Setup {
        #[arg(long, default_value = "0.0.31")]
        ekr_version: String,
    },
    /// Without an instance, print the JSON Schema of the spec file (`cortex.instance.InstanceSpec`).
    ///
    /// With one, have the instance's model propose ontology changes from a sample of the store's
    /// facts and their evidence, in batches of 20, and apply each change EKR applies as a schema
    /// transaction of its own; every proposal is written to `schema/<UTC stamp>/proposals.jsonl`
    /// (`cortex.instance.ProposeSchemaChanges`).
    #[command(verbatim_doc_comment)]
    Schema {
        name: Option<String>,
        /// How many facts to draw, 1 to 1000.
        #[arg(long, default_value_t = 40, requires = "name")]
        sample: i64,
        /// Record the proposals without applying any.
        #[arg(long, requires = "name")]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum SourceCommand {
    /// Add a source to an instance (`cortex.instance.AddSource`); `create` adds the spec's sources.
    Add {
        #[arg(long)]
        instance_name: String,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        name: String,
        #[arg(long, value_parser = ["Web", "Connectors", "Files", "Structured"])]
        kind: String,
        #[arg(long)]
        schedule: String,
    },
    /// Count a failed run; the second in a row disables the source
    /// (`cortex.instance.RecordFailure`).
    RecordFailure {
        source_id: String,
        #[arg(long)]
        reason: String,
    },
    /// Enable a disabled source again (`cortex.instance.EnableSource`).
    Enable { source_id: String },
}

const SPEC_SCHEMA: &str =
    include_str!("../generated/schema/schema/types/cortex.instance.InstanceSpec.schema.json");

/// How a command ended: the declared outcome, and the exit status it maps to.
struct Done {
    outcome: &'static str,
    ok: bool,
    detail: Value,
}

fn print(command: &str, done: &Done) -> ExitCode {
    println!(
        "{}",
        json!({"command": command, "outcome": done.outcome, "detail": done.detail})
    );
    if done.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("cortex: {message}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let home = Home::new(cli.home.clone().unwrap_or_else(Home::default_root));
    if let Some(reason) = schedule::home_refusal(&home.root) {
        return fail(reason);
    }
    match cli.command {
        Command::Schema { name: None, .. } => {
            println!("{SPEC_SCHEMA}");
            ExitCode::SUCCESS
        }
        Command::Setup { ekr_version } => match ekr::install(&ekr_version) {
            Ok(bin) => {
                println!("{}", json!({"ekr": bin}));
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Command::McpLine { name } => mcp_line(&home, &name),
        command => {
            // `restore` does not wait for the lock: while another command holds it, it answers
            // `busy` and writes nothing.
            let lock = match &command {
                Command::Restore { .. } => home.try_lock(),
                _ => home.lock().map(Some),
            };
            let lock = match lock {
                Ok(lock) => lock,
                Err(e) => return fail(format!("cannot lock {}: {e}", home.root.display())),
            };
            let registry = match home.load_registry() {
                Ok(r) => r,
                Err(e) => return fail(e),
            };
            let tools = || run::Tools {
                connectors: Connectors {
                    bin: cli.connectors.clone(),
                },
                claude: cli.claude.clone(),
                codex: cli.codex.clone(),
            };
            let shared: SharedRef = Rc::new(RefCell::new(Shared {
                registry,
                ..Shared::default()
            }));
            let mut app = Cortex::new(Generated::new(Ports {
                home: Home::new(home.root.clone()),
                tools: tools(),
                shared: shared.clone(),
                runner: None,
            }));
            let ctx = Ctx {
                home: &home,
                shared: shared.clone(),
                tools: tools(),
                locked: lock.is_some(),
                lock: RefCell::new(lock),
            };
            let result = dispatch(&mut app, &ctx, command);
            // A command that let the lock go early (`quality`) writes no registry: another command
            // may have written it since.
            if ctx.lock.borrow().is_some() {
                if let Err(e) = home.save_registry(&shared.borrow().registry) {
                    return fail(e);
                }
            }
            result
        }
    }
}

type App = Cortex<Generated<Ports>>;

/// What every command handler reads besides the generated behaviours.
struct Ctx<'a> {
    home: &'a Home,
    shared: SharedRef,
    tools: run::Tools,
    /// Whether this command took the home's lock; only `restore` runs without it.
    locked: bool,
    /// The home's lock while this command holds it. `quality` lets it go once its sample is drawn.
    lock: RefCell<Option<std::fs::File>>,
}

fn dispatch(app: &mut App, ctx: &Ctx, command: Command) -> ExitCode {
    match command {
        Command::Create {
            spec,
            no_extract,
            no_units,
            replace_binary,
            postgres_schema_config,
        } => create(
            app,
            ctx,
            &spec,
            no_extract,
            Units::of(no_units, replace_binary),
            postgres_schema_config.as_deref(),
        ),
        Command::Adopt {
            spec,
            store,
            host,
            seen,
            no_units,
            replace_binary,
        } => adopt(
            app,
            ctx,
            &Adoption {
                spec: &spec,
                store: &store,
                host: host.as_deref(),
                seen: &seen,
                units: Units::of(no_units, replace_binary),
            },
        ),
        Command::Update {
            name,
            spec,
            no_units,
            replace_binary,
        } => update(app, ctx, &name, &spec, Units::of(no_units, replace_binary)),
        Command::Remove { name } => remove(app, ctx, &name),
        Command::Restore { name, snapshot } => restore(app, ctx, &name, &snapshot),
        Command::Quality { name, sample } => quality(app, ctx, &name, sample),
        Command::Schema {
            name: Some(name),
            sample,
            dry_run,
        } => propose_schema(app, ctx, &name, sample, dry_run),
        Command::Run {
            source_id,
            record_failure,
        } => run_source(app, ctx, &source_id, record_failure),
        Command::Source { command } => match command {
            SourceCommand::Add {
                instance_name,
                source_id,
                name,
                kind,
                schedule,
            } => {
                let kind = match kind.as_str() {
                    "Web" => m::SourceKind::Web,
                    "Connectors" => m::SourceKind::Connectors,
                    "Structured" => m::SourceKind::Structured,
                    _ => m::SourceKind::Files,
                };
                let input = m::AddSource {
                    instance_name: m::InstanceName(instance_name),
                    source_id: m::SourceId(source_id),
                    name,
                    kind,
                    schedule,
                };
                match app.add_source(input) {
                    Ok(m::AddSourceOutcome::Added { source_added }) => print(
                        "source add",
                        &Done {
                            outcome: "added",
                            ok: true,
                            detail: json!({"source_id": source_added.source_id.0}),
                        },
                    ),
                    Err(e) => fail(e),
                }
            }
            SourceCommand::RecordFailure { source_id, reason } => {
                record_failure(app, ctx, &source_id, &reason)
            }
            SourceCommand::Enable { source_id } => enable(app, ctx, &source_id),
        },
        Command::List => match app.instances() {
            Ok(rows) => {
                // The version of `<home>/bin/cortex`, which every source unit of the home runs.
                let binary_version = schedule::binary_version(&ctx.home.root);
                for row in rows {
                    let layout = Layout::new(ctx.home.instance_dir(&row.name.0));
                    let port = layout.load_meta().map(|m| m.view_port).ok();
                    println!(
                        "{}",
                        json!({
                            "name": row.name.0,
                            "state": home::instance_state_name(row.state),
                            "description": row.description,
                            "model": row.model,
                            "ekr_version": row.ekr_version,
                            "seed_digest": ctx.home.frozen_seed_digest(&row.name.0).ok(),
                            "view": port.map(|p| format!("http://127.0.0.1:{p}/")),
                            "binary_version": binary_version,
                        })
                    );
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Command::Sources => match app.sources() {
            Ok(rows) => {
                for row in rows {
                    println!(
                        "{}",
                        json!({
                            "source_id": row.source_id.0,
                            "kind": home::kind_name(row.kind),
                            "state": home::source_state_name(row.state),
                            "schedule": row.schedule,
                            "runs": row.runs,
                            "consecutive_failures": row.consecutive_failures,
                        })
                    );
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Command::Schema { name: None, .. } | Command::Setup { .. } | Command::McpLine { .. } => {
            unreachable!("handled before the lock")
        }
    }
}

/// Every connection the spec names that Connectors does not list, or lists as revoked.
fn missing_connection(ctx: &Ctx, spec: &m::InstanceSpec) -> Result<Option<String>, String> {
    let tools = &ctx.tools;
    for (adapter, connection) in spec::connections(spec) {
        match tools.connectors.connection_state(&adapter, &connection) {
            Ok(Some(state)) if state != "revoked" => {}
            Ok(_) => return Ok(Some(format!("{adapter}:{connection}"))),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(None)
}

/// Seeds a new instance directory: frozen spec, host, store, seed schema. Answers why it failed.
/// A `postgres` store is provisioned first when `schema_config` names the schema-management role.
fn seed_instance(
    layout: &Layout,
    loaded: &spec::Loaded,
    schema_config: Option<&Path>,
) -> Result<(), String> {
    let spec = &loaded.model;
    if let Some(reason) = store_refusal(spec) {
        return Err(reason);
    }
    std::fs::create_dir_all(&layout.dir).map_err(|e| e.to_string())?;
    layout.freeze(loaded)?;
    let bin = ekr::resolve_bin(&spec.ekr.version, spec.ekr.bin.as_deref());
    if !bin.is_file() {
        return Err(format!(
            "ekr {} is not at {} (run `cortex setup --ekr-version {}`)",
            spec.ekr.version,
            bin.display(),
            spec.ekr.version
        ));
    }
    let host = ekr::Binary(bin).host_json(&spec.name.0)?;
    home::write_atomic(&layout.host(), host.as_bytes())?;
    let store = layout.store_handle(spec);
    let seed_path = match &spec.seed.ekr_seed {
        Some(rel) => layout.dir.join(rel),
        None => {
            let path = layout.dir.join("seed.yaml");
            home::write_atomic(&path, ekr::MINIMAL_SEED.as_bytes())?;
            path
        }
    };
    let schema = spec.seed.schema.as_ref().map(|s| layout.dir.join(s));
    if store.backend == ekr::Backend::Sqlite {
        return seed_and_apply(&store, &seed_path, schema.as_deref());
    }

    // PostgreSQL outlives the instance directory a refusal deletes, so nothing is written to it
    // that could be refused.
    let tenant = &spec.name.0;
    let held = || {
        format!(
            "tenant {tenant:?} already holds a store in PostgreSQL: cortex creates only new \
             stores; taking over an existing one is story:adopt-existing-store"
        )
    };
    if let Some(schema_config) = schema_config {
        store.provision(schema_config)?;
    }
    if store.seeded()? {
        return Err(held());
    }
    // The seed and the seed schema are tried on a scratch SQLite store first, by the same `ekr`.
    let scratch = ekr::Store {
        bin: store.bin.clone(),
        host: store.host.clone(),
        backend: ekr::Backend::Sqlite,
        store: layout.dir.join("seed-check.sqlite"),
    };
    let checked = seed_and_apply(&scratch, &seed_path, schema.as_deref());
    remove_scratch(&layout.dir, "seed-check.sqlite");
    checked.map_err(|e| {
        format!("{e} (found on a scratch SQLite store; no PostgreSQL lineage was written)")
    })?;
    let left = |e: String| match store.seeded() {
        Ok(false) => format!("{e} (no PostgreSQL lineage was written)"),
        _ => format!(
            "{e} (the PostgreSQL lineage of tenant {tenant:?} remains; drop it before creating \
             this instance again)"
        ),
    };
    // Another home may seed the tenant between the check above and this seed, and an identical
    // seed answers that home's result with exit 0. `ekr` stamps a seed it writes after this
    // clock reading, so an older `committed_at` is the other home's seed.
    let started = unix_millis();
    let committed_at = store.seed(&seed_path).map_err(left)?;
    if committed_at < started {
        return Err(held());
    }
    apply_schema(&store, schema.as_deref()).map_err(left)
}

fn unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Seeds `store` and applies the seed schema, refusing a schema with rejected parts.
fn seed_and_apply(store: &ekr::Store, seed: &Path, schema: Option<&Path>) -> Result<(), String> {
    store.seed(seed)?;
    apply_schema(store, schema)
}

/// Applies the seed schema, refusing a schema with rejected parts.
fn apply_schema(store: &ekr::Store, schema: Option<&Path>) -> Result<(), String> {
    if let Some(schema) = schema {
        let report = store.apply(schema)?;
        let applied = ekr::applied(&report);
        if applied.rejected > 0 {
            return Err(format!(
                "the seed schema had {} rejected part(s): {report}",
                applied.rejected
            ));
        }
    }
    Ok(())
}

/// Removes the SQLite file `name` in `dir` and its `-wal`, `-shm` and `-journal` companions.
fn remove_scratch(dir: &Path, name: &str) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(dir.join(format!("{name}{suffix}")));
    }
}

/// Why the spec's `store` cannot be used, naming the field and never its value, which may be a
/// connection string an operator wrote in the wrong place.
fn store_refusal(spec: &m::InstanceSpec) -> Option<String> {
    match &spec.store {
        // Timers and the viewer run from no particular directory, so the path must stand alone.
        Some(m::StoreSpec::Postgres(p)) if !ekr::expand(&p.config).is_absolute() => Some(
            "store.value.config: write an absolute path, or one starting with ~/, to an \
             ekr.postgres/1 file"
                .into(),
        ),
        Some(m::StoreSpec::Sqlite(Some(m::SqliteStore { path: Some(_) }))) => Some(
            "store.value.path is not supported yet: a sqlite store is store.sqlite in the \
             instance directory; remove the path"
                .into(),
        ),
        _ => None,
    }
}

/// Whether a command installs systemd units (`--no-units`), and whether it may replace a newer
/// `<home>/bin/cortex` (`--replace-binary`).
#[derive(Clone, Copy)]
enum Units {
    None,
    Install { replace_binary: bool },
}

impl Units {
    fn of(no_units: bool, replace_binary: bool) -> Self {
        if no_units {
            Self::None
        } else {
            Self::Install { replace_binary }
        }
    }
}

/// Why `units` cannot be installed for `name`: another home wrote a unit of the same name
/// ([`Systemd::foreign_units`]). Checked before a command changes anything.
fn foreign_units(
    ctx: &Ctx,
    name: &str,
    spec: &m::InstanceSpec,
    units: Units,
    view: bool,
) -> Option<String> {
    match units {
        Units::None => None,
        Units::Install { .. } => {
            let sources: Vec<String> = spec.sources.iter().map(|s| s.name.clone()).collect();
            Systemd::from_env(&ctx.home.root).foreign_units(name, &sources, view)
        }
    }
}

/// Puts what installing the units did in `detail["units"]`: each unit taken over from a home that
/// holds no instance of the name ([`Systemd::taken_over`]), and why the install failed, when it
/// did.
fn units_detail(systemd: &Systemd, installed: Result<(), String>, detail: &mut Value) {
    let taken = systemd.taken_over.take();
    if !taken.is_empty() {
        detail["units"]["taken_over"] = json!(taken);
    }
    if let Err(e) = installed {
        detail["units"]["failed"] = json!(e);
    }
}

/// Places this cortex at `<home>/bin/cortex` ([`Systemd::place_binary`]), puts what that did in
/// `detail["binary"]`, then installs each source's units. A spec with no source installs no unit
/// that runs cortex, so it leaves the binary alone.
fn install_sources(
    ctx: &Ctx,
    systemd: &Systemd,
    name: &str,
    spec: &m::InstanceSpec,
    replace_binary: bool,
    detail: &mut Value,
) -> Result<(), String> {
    if spec.sources.is_empty() {
        return Ok(());
    }
    let placed = systemd.place_binary(replace_binary)?;
    if let Some(reason) = placed.reason.as_ref().filter(|_| !placed.replaced) {
        eprintln!("cortex: {reason}");
    }
    detail["binary"] = placed.json();
    spec.sources
        .iter()
        .try_for_each(|s| systemd.install_source(name, &s.name, &s.schedule, &ctx.tools))
}

fn create(
    app: &mut App,
    ctx: &Ctx,
    path: &Path,
    no_extract: bool,
    units: Units,
    schema_config: Option<&Path>,
) -> ExitCode {
    let loaded = match spec::load_given(path) {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let spec = loaded.model.clone();
    if schema_config.is_some() && !matches!(spec.store, Some(m::StoreSpec::Postgres(_))) {
        return fail("--postgres-schema-config needs `store.backend: postgres` in the spec");
    }
    let name = spec.name.0.clone();
    if let Some(reason) = foreign_units(ctx, &name, &spec, units, true) {
        return fail(reason);
    }
    let layout = Layout::new(ctx.home.instance_dir(&name));
    const CMD: &str = "cortex.instance.CreateInstance";
    let lineage = instance::Lineage::for_tenant(&spec, &name);

    // `name-taken` is the generated behaviour's own lookup; a taken name skips the checks below.
    let taken = ctx
        .shared
        .borrow_mut()
        .registry
        .instances
        .contains_key(&name);
    // The seed documents' extraction, run before the instance is recorded so that a seed that
    // stops early is the declared `partial`.
    let mut seeded: Option<Result<run::Report, run::Failure>> = None;
    if !taken {
        match missing_connection(ctx, &spec) {
            Ok(Some(missing)) => {
                ctx.shared
                    .borrow_mut()
                    .external
                    .insert((CMD, "connection-missing"), true);
                ctx.shared.borrow_mut().strings.push_back(missing);
            }
            Ok(None) => {
                // A `postgres` store's tenant is the instance's name; another instance (one
                // adopted under that tenant) may already grow that lineage.
                let held = lineage.as_ref().and_then(|l| {
                    held_lineages(ctx)
                        .into_iter()
                        .find(|(_, h)| h == l)
                        .map(|(holder, _)| holder)
                });
                if let Some(holder) = held {
                    ctx.shared
                        .borrow_mut()
                        .external
                        .insert((CMD, "seed-refused"), true);
                    ctx.shared.borrow_mut().strings.push_back(format!(
                        "instance {holder:?} already grows this PostgreSQL lineage: the same \
                         store.value.config and tenant {name:?}"
                    ));
                } else if layout.dir.exists() {
                    ctx.shared
                        .borrow_mut()
                        .external
                        .insert((CMD, "seed-refused"), true);
                    ctx.shared.borrow_mut().strings.push_back(format!(
                        "{} exists and is no registered instance",
                        layout.dir.display()
                    ));
                } else if let Err(reason) = seed_instance(&layout, &loaded, schema_config) {
                    let _ = std::fs::remove_dir_all(&layout.dir);
                    ctx.shared
                        .borrow_mut()
                        .external
                        .insert((CMD, "seed-refused"), true);
                    ctx.shared.borrow_mut().strings.push_back(reason);
                } else if !no_extract && !spec.seed.documents.is_empty() {
                    let ran = run::seed(&layout, &ctx.tools);
                    if !seed_finished(&ran) {
                        ctx.shared
                            .borrow_mut()
                            .external
                            .insert((CMD, "partial"), true);
                    }
                    seeded = Some(ran);
                }
            }
            Err(e) => return fail(e),
        }
    }
    let port = match spec.serve.view_port {
        Some(p) => p,
        None => instance::free_port(18900).map(i64::from).unwrap_or(0),
    };
    ctx.shared.borrow_mut().integers.push_back(port);
    let input = m::CreateInstance {
        name: spec.name.clone(),
        description: spec.description.clone(),
        model: spec.model.model.clone(),
        ekr_version: spec.ekr.version.clone(),
        // A path `freeze` refuses is never hashed; `seed-refused` answers it below.
        seed_digest: match home::seed_path_refusal(&spec) {
            Some(_) => String::new(),
            None => home::seed_digest(&loaded.dir, &spec),
        },
        spec: spec.clone(),
    };
    let outcome = match app.create_instance(input) {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    let done = match outcome {
        m::CreateInstanceOutcome::NameTaken { error } => Done {
            outcome: "name-taken",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
        m::CreateInstanceOutcome::ConnectionMissing { error } => Done {
            outcome: "connection-missing",
            ok: false,
            detail: json!({
                "connection": error.connection,
                "next": "connectors connections connect --adapter <adapter> --profile <profile> --credential-prompt",
            }),
        },
        m::CreateInstanceOutcome::SeedRefused { error } => Done {
            outcome: "seed-refused",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        m::CreateInstanceOutcome::Created { instance_created }
        | m::CreateInstanceOutcome::Partial { instance_created } => {
            let partial = seeded.as_ref().is_some_and(|ran| !seed_finished(ran));
            let port = instance_created.view_port as u16;
            let meta = Meta {
                format: "cortex.instance-meta/1".into(),
                spec_dir: std::fs::canonicalize(&loaded.dir).unwrap_or(loaded.dir.clone()),
                view_port: port,
                lineage,
                adopted: false,
            };
            if let Err(e) = layout.save_meta(&meta) {
                return fail(e);
            }
            for source in &spec.sources {
                let input = m::AddSource {
                    instance_name: spec.name.clone(),
                    source_id: m::SourceId(format!("{name}/{}", source.name)),
                    name: source.name.clone(),
                    kind: model_map::kind(&source.settings),
                    schedule: source.schedule.clone(),
                };
                if let Err(e) = app.add_source(input) {
                    return fail(e);
                }
            }
            let mut detail = json!({
                "name": name,
                "dir": layout.dir,
                "view": format!("http://127.0.0.1:{port}/"),
                "sources": spec.sources.iter().map(|s| format!("{name}/{}", s.name)).collect::<Vec<_>>(),
            });
            if let Some(ran) = &seeded {
                detail["seed"] = match ran {
                    Ok(r) => {
                        let mut seed = json!({
                            "documents_applied": r.documents_applied,
                            "cost_usd": r.cost_usd,
                        });
                        // A seed that stopped early says what it left and why; one that finished
                        // reports as before.
                        if partial {
                            seed["documents_new"] = json!(r.documents_new);
                            seed["stopped"] = json!(r.stopped);
                            seed["facts_refused"] = json!(r.facts_refused);
                            seed["parts_rejected"] = json!(r.parts_rejected);
                        }
                        // As `cortex run` reports it: present only when the policy pseudonymises.
                        if let Some(redacted) = &r.redacted {
                            seed["redacted"] = json!(redacted);
                            seed["unrestored"] = json!(r.unrestored);
                        }
                        seed
                    }
                    Err(e) => json!({"failed": format!("{e:?}")}),
                };
            }
            if partial {
                eprintln!(
                    "cortex: the seed stopped before every seed document was extracted; \
                     `cortex run {name}/seed` extracts the rest"
                );
            }
            if let Units::Install { replace_binary } = units {
                let systemd = Systemd::from_env(&ctx.home.root);
                let store = layout.store_handle(&spec);
                let installed = systemd.install_view(&name, &store, port).and_then(|_| {
                    install_sources(ctx, &systemd, &name, &spec, replace_binary, &mut detail)
                });
                units_detail(&systemd, installed, &mut detail);
            }
            Done {
                outcome: if partial { "partial" } else { "created" },
                ok: true,
                detail,
            }
        }
    };
    print("create", &done)
}

/// Whether a seed extraction extracted every seed document it wanted: it failed, or stopped
/// before the last batch (the budget spent, a model call or an apply failed), when not.
fn seed_finished(ran: &Result<run::Report, run::Failure>) -> bool {
    matches!(ran, Ok(r) if r.documents_applied >= r.documents_new)
}

/// `--seen <source>=<file>`.
fn seen_arg(text: &str) -> Result<(String, PathBuf), String> {
    match text.split_once('=') {
        Some((source, file)) if !source.is_empty() && !file.is_empty() => {
            Ok((source.to_string(), PathBuf::from(file)))
        }
        _ => Err("write <source>=<file>".into()),
    }
}

/// The arguments of `cortex adopt`.
struct Adoption<'a> {
    spec: &'a Path,
    store: &'a Path,
    host: Option<&'a Path>,
    seen: &'a [(String, PathBuf)],
    units: Units,
}

/// Why an adoption took no instance: a declared refusal, or a failure before any outcome.
enum NotAdopted {
    Refused(&'static str, String),
    Failed(String),
}

/// What an adoption found in the store.
struct Adopted {
    revision: i64,
    lineage: Option<instance::Lineage>,
    /// Every document the `--seen` files name.
    seen_documents: usize,
    /// Those of them the store holds no evidence of: they are not extracted again all the same.
    seen_without_evidence: usize,
}

/// The PostgreSQL lineage of every active instance of the home, by instance name.
fn held_lineages(ctx: &Ctx) -> Vec<(String, instance::Lineage)> {
    let names: Vec<String> = ctx
        .shared
        .borrow()
        .registry
        .instances
        .values()
        .filter(|i| i.state == m::InstanceState::Active)
        .map(|i| i.data.name.0.clone())
        .collect();
    names
        .into_iter()
        .filter_map(|n| {
            let lineage = Layout::new(ctx.home.instance_dir(&n)).lineage()?;
            Some((n, lineage))
        })
        .collect()
}

/// Fills a new instance directory from an existing store: the frozen spec, the host, the store's
/// copy (SQLite) and each source's seen state. Nothing is written to the store named, beyond the
/// side files SQLite writes beside a database it opens, and no seed or extraction runs.
fn adopt_into(
    layout: &Layout,
    loaded: &spec::Loaded,
    a: &Adoption,
    seen: &[(String, cortex_cli::state::SeenState)],
    ctx: &Ctx,
) -> Result<Adopted, NotAdopted> {
    use instance::{AdoptRefusal, CopyError};
    let spec = &loaded.model;
    // Every check below reads the file `--store` resolves to, never a symlink beside it.
    let from = std::fs::canonicalize(a.store).map_err(|e| {
        NotAdopted::Refused("store-unreadable", format!("cannot read --store: {e}"))
    })?;
    let backend = instance::adopt_backend(spec, &from).map_err(|r| match r {
        AdoptRefusal::BackendMismatch(reason) => NotAdopted::Refused("backend-mismatch", reason),
        AdoptRefusal::StoreUnreadable(reason) => NotAdopted::Refused("store-unreadable", reason),
    })?;
    let failed = NotAdopted::Failed;
    let bin = ekr::resolve_bin(&spec.ekr.version, spec.ekr.bin.as_deref());
    if !bin.is_file() {
        return Err(failed(format!(
            "ekr {} is not at {} (run `cortex setup --ekr-version {}`)",
            spec.ekr.version,
            bin.display(),
            spec.ekr.version
        )));
    }
    let host = match a.host {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| failed(format!("cannot read --host {}: {e}", path.display())))?,
        None => ekr::Binary(bin).host_json(&spec.name.0).map_err(failed)?,
    };
    // Every run names the host's operator as the evidence's `extracted_by`.
    ekr::operator(&host).map_err(|e| failed(format!("--host: {e}")))?;
    let lineage = instance::Lineage::of(spec, &host);
    if let Some(lineage) = &lineage {
        if let Some((holder, _)) = held_lineages(ctx).into_iter().find(|(_, l)| l == lineage) {
            return Err(NotAdopted::Refused("store-held", holder));
        }
    }
    if backend == ekr::Backend::Sqlite {
        let need = instance::copy_needs(&from).map_err(|e| {
            NotAdopted::Refused("store-unreadable", format!("cannot read --store: {e}"))
        })?;
        let free = instance::free_bytes(&ctx.home.root)
            .map_err(|e| failed(format!("cannot read free space under the home: {e}")))?;
        if let Some(refusal) = instance::space_refusal(&ctx.home.root, need, free) {
            return Err(failed(refusal));
        }
    }
    std::fs::create_dir_all(&layout.dir).map_err(|e| failed(e.to_string()))?;
    layout.freeze(loaded).map_err(failed)?;
    home::write_atomic(&layout.host(), host.as_bytes()).map_err(failed)?;
    let store = layout.store_handle(spec);
    if backend == ekr::Backend::Sqlite {
        instance::copy_store(&from, &store.store).map_err(|e| match e {
            CopyError::Source(reason) => NotAdopted::Refused("store-unreadable", reason),
            CopyError::Destination(reason) => failed(reason),
        })?;
    }
    let ontology = store
        .ontology()
        .map_err(|e| NotAdopted::Refused("store-unreadable", e))?;
    let revision = ontology["revision"].as_i64().ok_or_else(|| {
        NotAdopted::Refused(
            "store-unreadable",
            "ekr ontology answered no revision".into(),
        )
    })?;
    let types = instance::seed_types(&layout.dir, spec).map_err(failed)?;
    let missing = instance::missing_types(&types, &ontology);
    if !missing.is_empty() {
        return Err(NotAdopted::Refused(
            "seed-types-missing",
            missing.join(", "),
        ));
    }
    let seen_documents: usize = seen.iter().map(|(_, s)| s.documents.len()).sum();
    let mut seen_without_evidence = 0;
    if seen_documents > 0 {
        let identities = instance::evidence_identities(&store)
            .map_err(|e| NotAdopted::Refused("store-unreadable", e))?;
        seen_without_evidence = seen
            .iter()
            .flat_map(|(_, s)| s.documents.keys())
            .filter(|key| !instance::has_evidence(key, &identities))
            .count();
    }
    for (source, state) in seen {
        state.save(&layout.seen(source)).map_err(failed)?;
    }
    Ok(Adopted {
        revision,
        lineage,
        seen_documents,
        seen_without_evidence,
    })
}

fn adopt(app: &mut App, ctx: &Ctx, a: &Adoption) -> ExitCode {
    const CMD: &str = "cortex.instance.AdoptInstance";
    let loaded = match spec::load_given(a.spec) {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let spec = loaded.model.clone();
    if let Some(reason) = store_refusal(&spec).or_else(|| home::seed_path_refusal(&spec)) {
        return fail(reason);
    }
    let mut seen: Vec<(String, cortex_cli::state::SeenState)> = Vec::new();
    for (source, file) in a.seen {
        if seen.iter().any(|(s, _)| s == source) {
            return fail(format!(
                "--seen {source}: given twice; name one cortex.seen/1 file per source"
            ));
        }
        if !spec.sources.iter().any(|s| &s.name == source) {
            return fail(format!(
                "--seen {source}: the spec has no source {source:?}"
            ));
        }
        match cortex_cli::state::SeenState::read_document(file) {
            Ok(state) => seen.push((source.clone(), state)),
            Err(e) => return fail(format!("--seen {source}: {e}")),
        }
    }
    let name = spec.name.0.clone();
    if let Some(reason) = foreign_units(ctx, &name, &spec, a.units, true) {
        return fail(reason);
    }
    let layout = Layout::new(ctx.home.instance_dir(&name));

    // `name-taken` is the generated behaviour's own lookup; a taken name skips the checks below.
    let taken = ctx.shared.borrow().registry.instances.contains_key(&name);
    let mut adopted = None;
    if !taken {
        if layout.dir.exists() {
            return fail(format!(
                "{} exists and is no registered instance",
                layout.dir.display()
            ));
        }
        match adopt_into(&layout, &loaded, a, &seen, ctx) {
            Ok(found) => {
                ctx.shared.borrow_mut().integers.push_back(found.revision);
                adopted = Some(found);
            }
            Err(not) => {
                let _ = std::fs::remove_dir_all(&layout.dir);
                match not {
                    NotAdopted::Failed(e) => return fail(e),
                    NotAdopted::Refused(outcome, reason) => {
                        let mut shared = ctx.shared.borrow_mut();
                        shared.external.insert((CMD, outcome), true);
                        shared.strings.push_back(reason);
                    }
                }
            }
        }
    }
    let port = match spec.serve.view_port {
        Some(p) => p,
        None => instance::free_port(18900).map(i64::from).unwrap_or(0),
    };
    ctx.shared.borrow_mut().integers.push_back(port);
    let input = m::AdoptInstance {
        name: spec.name.clone(),
        description: spec.description.clone(),
        model: spec.model.model.clone(),
        ekr_version: spec.ekr.version.clone(),
        seed_digest: home::seed_digest(&loaded.dir, &spec),
        spec: spec.clone(),
        store: a.store.display().to_string(),
    };
    let outcome = match app.adopt_instance(input) {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    let done = match outcome {
        m::AdoptInstanceOutcome::NameTaken { error } => Done {
            outcome: "name-taken",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
        m::AdoptInstanceOutcome::BackendMismatch { error } => Done {
            outcome: "backend-mismatch",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        m::AdoptInstanceOutcome::StoreUnreadable { error } => Done {
            outcome: "store-unreadable",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        m::AdoptInstanceOutcome::StoreHeld { error } => Done {
            outcome: "store-held",
            ok: false,
            detail: json!({"name": error.name}),
        },
        m::AdoptInstanceOutcome::SeedTypesMissing { error } => Done {
            outcome: "seed-types-missing",
            ok: false,
            detail: json!({"types": error.types}),
        },
        m::AdoptInstanceOutcome::Adopted { instance_adopted } => {
            let Some(found) = adopted else {
                return fail("adopted without reading the store");
            };
            let port = instance_adopted.view_port as u16;
            let meta = Meta {
                format: "cortex.instance-meta/1".into(),
                spec_dir: std::fs::canonicalize(&loaded.dir).unwrap_or(loaded.dir.clone()),
                view_port: port,
                lineage: found.lineage,
                adopted: true,
            };
            if let Err(e) = layout.save_meta(&meta) {
                return fail(e);
            }
            for source in &spec.sources {
                let input = m::AddSource {
                    instance_name: spec.name.clone(),
                    source_id: m::SourceId(format!("{name}/{}", source.name)),
                    name: source.name.clone(),
                    kind: model_map::kind(&source.settings),
                    schedule: source.schedule.clone(),
                };
                if let Err(e) = app.add_source(input) {
                    return fail(e);
                }
            }
            let mut detail = json!({
                "name": name,
                "dir": layout.dir,
                "revision": instance_adopted.revision,
                "view": format!("http://127.0.0.1:{port}/"),
                "sources": spec.sources.iter().map(|s| format!("{name}/{}", s.name)).collect::<Vec<_>>(),
                "seen": seen.iter().map(|(s, _)| format!("{name}/{s}")).collect::<Vec<_>>(),
                "seen_documents": found.seen_documents,
                "seen_without_evidence": found.seen_without_evidence,
            });
            if let Units::Install { replace_binary } = a.units {
                let systemd = Systemd::from_env(&ctx.home.root);
                let store = layout.store_handle(&spec);
                let installed = systemd.install_view(&name, &store, port).and_then(|_| {
                    install_sources(ctx, &systemd, &name, &spec, replace_binary, &mut detail)
                });
                units_detail(&systemd, installed, &mut detail);
            }
            Done {
                outcome: "adopted",
                ok: true,
                detail,
            }
        }
    };
    print("adopt", &done)
}

fn update(app: &mut App, ctx: &Ctx, name: &str, path: &Path, units: Units) -> ExitCode {
    let loaded = match spec::load_given(path) {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let spec = loaded.model.clone();
    if spec.name.0 != name {
        return fail(format!("the spec names {:?}, not {name:?}", spec.name.0));
    }
    if let Some(reason) = foreign_units(ctx, name, &spec, units, false) {
        return fail(reason);
    }
    let layout = Layout::new(ctx.home.instance_dir(name));
    // A store the spec cannot use, another store than the instance's (its history is in the store
    // it has), or a seed or instructions path `create` would refuse is a seed change. The input then
    // carries the refusal, which no stored digest equals, and a refused path is never hashed. The
    // reason names fields only.
    let mut refusal = store_refusal(&spec).or_else(|| home::seed_path_refusal(&spec));
    if refusal.is_none() {
        if let Ok(old) = layout.load_spec() {
            let (was, will) = (layout.store_handle(&old), layout.store_handle(&spec));
            if (was.backend, was.store) != (will.backend, will.store) {
                refusal = Some(
                    "store: the spec names another store than the instance's; moving a store \
                     is not supported"
                        .into(),
                );
            }
        }
    }
    let seed_digest = match &refusal {
        Some(reason) => format!("refused: {reason}"),
        None => home::seed_digest(&loaded.dir, &spec),
    };
    // The stored seed is the frozen copy, read only here. A frozen spec that is missing or cannot
    // be parsed fails the update, except for a removed instance, which holds no seed to compare
    // once its directory is gone and answers `not-active`.
    if let Some(held) = ctx.shared.borrow_mut().registry.instances.get_mut(name) {
        held.data.seed_digest = match ctx.home.frozen_seed_digest(name) {
            Ok(frozen) => frozen,
            // A refusal known from the new spec alone answers before the frozen copy is needed.
            Err(_) if refusal.is_some() => String::new(),
            Err(_) if held.state == m::InstanceState::Removed => seed_digest.clone(),
            Err(e) => return fail(e),
        };
    }
    let input = m::UpdateInstance {
        name: spec.name.clone(),
        description: spec.description.clone(),
        model: spec.model.model.clone(),
        spec: spec.clone(),
        seed_digest,
    };
    let done = match app.update_instance(input) {
        Err(e) => return fail(e),
        Ok(m::UpdateInstanceOutcome::SeedChangeRefused { error }) => Done {
            outcome: "seed-change-refused",
            ok: false,
            detail: match &refusal {
                Some(reason) => json!({"name": error.name.0, "reason": reason}),
                None => json!({"name": error.name.0}),
            },
        },
        Ok(m::UpdateInstanceOutcome::NotActive { .. }) => Done {
            outcome: "not-active",
            ok: false,
            detail: json!({"name": name}),
        },
        Ok(m::UpdateInstanceOutcome::NoSuchInstance { error }) => Done {
            outcome: "no-such-instance",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
        Ok(m::UpdateInstanceOutcome::Updated { instance_updated }) => {
            if let Err(e) = home::write_atomic(&layout.spec(), loaded.text.as_bytes()) {
                return fail(e);
            }
            let mut added = Vec::new();
            for source in &spec.sources {
                let id = format!("{name}/{}", source.name);
                if ctx.shared.borrow_mut().registry.sources.contains_key(&id) {
                    continue;
                }
                let input = m::AddSource {
                    instance_name: spec.name.clone(),
                    source_id: m::SourceId(id.clone()),
                    name: source.name.clone(),
                    kind: model_map::kind(&source.settings),
                    schedule: source.schedule.clone(),
                };
                if let Err(e) = app.add_source(input) {
                    return fail(e);
                }
                added.push(id);
            }
            let mut detail = json!({"name": instance_updated.name.0, "added_sources": added});
            if let Units::Install { replace_binary } = units {
                let systemd = Systemd::from_env(&ctx.home.root);
                let installed =
                    install_sources(ctx, &systemd, name, &spec, replace_binary, &mut detail);
                units_detail(&systemd, installed, &mut detail);
            }
            Done {
                outcome: "updated",
                ok: true,
                detail,
            }
        }
    };
    print("update", &done)
}

fn remove(app: &mut App, ctx: &Ctx, name: &str) -> ExitCode {
    let done = match app.remove_instance(m::RemoveInstance {
        name: m::InstanceName(name.to_string()),
    }) {
        Err(e) => return fail(e),
        Ok(m::RemoveInstanceOutcome::Removed { instance_removed }) => {
            let sources: Vec<String> = ctx
                .shared
                .borrow_mut()
                .registry
                .sources
                .values()
                .filter(|s| s.data.instance_name.0 == name)
                .map(|s| s.data.name.clone())
                .collect();
            let mut detail =
                json!({"name": instance_removed.name.0, "dir": ctx.home.instance_dir(name)});
            match Systemd::from_env(&ctx.home.root).remove_instance(name, &sources) {
                Ok(kept) if !kept.is_empty() => detail["units"] = json!({"kept": kept}),
                Ok(_) => {}
                Err(e) => detail["units"] = json!({"failed": e}),
            }
            Done {
                outcome: "removed",
                ok: true,
                detail,
            }
        }
        Ok(m::RemoveInstanceOutcome::WrongState { .. }) => Done {
            outcome: "wrong-state",
            ok: false,
            detail: json!({"name": name}),
        },
        Ok(m::RemoveInstanceOutcome::NoSuchInstance { error }) => Done {
            outcome: "no-such-instance",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
    };
    print("remove", &done)
}

fn restore(app: &mut App, ctx: &Ctx, name: &str, snapshot: &str) -> ExitCode {
    const CMD: &str = "cortex.instance.RestoreSnapshot";
    let mut finished = None;
    // An unknown name restores nothing; the generated behaviour answers `no-such-instance`.
    let known = ctx.shared.borrow().registry.instances.contains_key(name);
    if known {
        let layout = Layout::new(ctx.home.instance_dir(name));
        let refusal = if ctx.locked {
            match snapshot::restore_held(
                &layout,
                name,
                snapshot,
                &Systemd::from_env(&ctx.home.root),
            ) {
                Ok(restored) => {
                    finished = Some(restored);
                    None
                }
                Err(RestoreError::Refused(refusal)) => Some(refusal),
                Err(RestoreError::Failed(e)) => return fail(e),
            }
        } else {
            Some(Refusal::Busy(format!(
                "another cortex command holds the lock of {}",
                ctx.home.root.display()
            )))
        };
        let mut shared = ctx.shared.borrow_mut();
        match refusal {
            None => {}
            Some(Refusal::Unsupported) => {
                shared.external.insert((CMD, "backend-unsupported"), true);
            }
            Some(Refusal::Busy(reason)) => {
                shared.external.insert((CMD, "busy"), true);
                shared.strings.push_back(reason);
            }
            Some(Refusal::NoSuchSnapshot) => {
                shared.external.insert((CMD, "no-such-snapshot"), true);
            }
        }
    }
    let done = match app.restore_snapshot(m::RestoreSnapshot {
        name: m::InstanceName(name.to_string()),
        snapshot: snapshot.to_string(),
    }) {
        Err(e) => return fail(e),
        Ok(m::RestoreSnapshotOutcome::Restored { snapshot_restored }) => {
            let mut detail = json!({
                "name": snapshot_restored.name.0,
                "snapshot": snapshot_restored.snapshot,
            });
            if let Some(restored) = finished {
                detail["before_restore"] = json!(restored.before_restore);
                detail["state_restored"] = json!(restored.state_restored);
                if let Some(e) = restored.viewer_failed {
                    detail["units"] = json!({"failed": e});
                }
            }
            Done {
                outcome: "restored",
                ok: true,
                detail,
            }
        }
        Ok(m::RestoreSnapshotOutcome::BackendUnsupported { error }) => Done {
            outcome: "backend-unsupported",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
        Ok(m::RestoreSnapshotOutcome::Busy { error }) => Done {
            outcome: "busy",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        Ok(m::RestoreSnapshotOutcome::NoSuchSnapshot { error }) => Done {
            outcome: "no-such-snapshot",
            ok: false,
            detail: json!({"snapshot": error.snapshot}),
        },
        Ok(m::RestoreSnapshotOutcome::NoSuchInstance { error }) => Done {
            outcome: "no-such-instance",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
    };
    print("restore", &done)
}

fn quality(app: &mut App, ctx: &Ctx, name: &str, sample: i64) -> ExitCode {
    const CMD: &str = "cortex.instance.MeasureQuality";
    // An unknown name measures nothing; the generated behaviour answers `no-such-instance`.
    let known = ctx.shared.borrow().registry.instances.contains_key(name);
    let mut measured = None;
    if known {
        let layout = Layout::new(ctx.home.instance_dir(name));
        // Only the draw reads the store, at one revision; the model calls and the files under
        // `quality/<stamp>/` need no lock, so scheduled runs do not wait behind the judge.
        let drawn = quality::draw(&layout, sample);
        drop(ctx.lock.borrow_mut().take());
        let result = drawn.and_then(|d| quality::judge(&layout, &ctx.tools, d));
        let mut shared = ctx.shared.borrow_mut();
        match result {
            Ok(q) => {
                shared.strings.push_back(q.stamp.clone());
                shared
                    .integers
                    .extend([q.revision, q.seed, q.judged, q.passed, q.unclear]);
                shared
                    .decimals
                    .extend([Decimal(q.lower.clone()), Decimal(q.upper.clone())]);
                measured = Some(q);
            }
            Err(quality::Failure::Sample(reason)) => {
                shared.external.insert((CMD, "sample-failed"), true);
                shared.strings.push_back(reason);
            }
            Err(quality::Failure::Judge(reason)) => {
                shared.external.insert((CMD, "judge-failed"), true);
                shared.strings.push_back(reason);
            }
        }
    }
    let number = |d: &Decimal| serde_json::from_str::<Value>(&d.0).unwrap_or(Value::Null);
    let done = match app.measure_quality(m::MeasureQuality {
        name: m::InstanceName(name.to_string()),
        sample,
    }) {
        Err(e) => return fail(e),
        Ok(m::MeasureQualityOutcome::Measured {
            quality_measured: e,
        }) => {
            let mut detail = json!({
                "name": e.name.0,
                "stamp": e.stamp,
                "revision": e.revision,
                "seed": e.seed,
                "judged": e.judged,
                "passed": e.passed,
                "unclear": e.unclear,
                "lower": number(&e.lower),
                "upper": number(&e.upper),
            });
            // Not on the event (`spec/domains/instance.yaml`, `UNMAPPED:` on `measured`).
            if let Some(q) = measured {
                detail["rate"] = q.rate;
                detail["cost_usd"] = json!(q.cost_usd.map(|c| format!("{c:.4}")));
                detail["dir"] = json!(q.dir);
            }
            Done {
                outcome: "measured",
                ok: true,
                detail,
            }
        }
        Ok(m::MeasureQualityOutcome::SampleFailed { error }) => Done {
            outcome: "sample-failed",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        Ok(m::MeasureQualityOutcome::JudgeFailed { error }) => Done {
            outcome: "judge-failed",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        Ok(m::MeasureQualityOutcome::NoSuchInstance { error }) => Done {
            outcome: "no-such-instance",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
    };
    print("quality", &done)
}

fn propose_schema(app: &mut App, ctx: &Ctx, name: &str, sample: i64, dry_run: bool) -> ExitCode {
    const CMD: &str = "cortex.instance.ProposeSchemaChanges";
    // An unknown name proposes nothing; the generated behaviour answers `no-such-instance`.
    let known = ctx.shared.borrow().registry.instances.contains_key(name);
    let mut proposed = None;
    if known {
        let layout = Layout::new(ctx.home.instance_dir(name));
        // As `quality`: only the draw reads the store; the model calls need no lock, so scheduled
        // runs do not wait behind them. Applying takes the lock again, and decides each proposal
        // against the ontology at the head then.
        let drawn = schema::draw(&layout, sample);
        drop(ctx.lock.borrow_mut().take());
        let asked = drawn.and_then(|d| schema::ask(&layout, &ctx.tools, d));
        let result = match asked {
            Err(e) => Err(e),
            Ok(asked) if dry_run => schema::record(asked, None),
            Ok(asked) => {
                // Held only while applying; the registry loaded before the model calls is not
                // written back (`main`), as another command may have written it since.
                let _lock = match ctx.home.lock() {
                    Ok(lock) => lock,
                    Err(e) => return fail(format!("cannot lock {}: {e}", ctx.home.root.display())),
                };
                match schema::target_of(&layout) {
                    Ok((store, operator)) => schema::record(
                        asked,
                        Some(schema::Target {
                            store: &store,
                            operator: &operator,
                        }),
                    ),
                    Err(e) => Err(schema::Failure::Propose(e)),
                }
            }
        };
        let mut shared = ctx.shared.borrow_mut();
        match result {
            Ok(p) => {
                shared.strings.push_back(p.stamp.clone());
                shared.integers.extend([
                    p.revision,
                    p.proposed,
                    p.applied,
                    p.refused,
                    p.recorded_only,
                    p.invalid,
                    p.dropped,
                ]);
                proposed = Some(p);
            }
            Err(schema::Failure::Sample(reason)) => {
                shared.external.insert((CMD, "sample-failed"), true);
                shared.strings.push_back(reason);
            }
            Err(schema::Failure::Propose(reason)) => {
                shared.external.insert((CMD, "propose-failed"), true);
                shared.strings.push_back(reason);
            }
        }
    }
    let detail = |e: &m::SchemaChangesProposed| {
        let mut detail = json!({
            "name": e.name.0,
            "stamp": e.stamp,
            "revision": e.revision,
            "proposed": e.proposed,
            "applied": e.applied,
            "refused": e.refused,
            "recorded_only": e.recorded_only,
            "invalid": e.invalid,
            "dropped": e.dropped,
        });
        // Not on the event (`spec/domains/instance.yaml`, `UNMAPPED:` on `proposed`).
        if let (Some(p), Some(o)) = (&proposed, detail.as_object_mut()) {
            o.extend(schema::extra(p));
        }
        detail
    };
    let done = match app.propose_schema_changes(m::ProposeSchemaChanges {
        name: m::InstanceName(name.to_string()),
        sample,
        dry_run,
    }) {
        Err(e) => return fail(e),
        Ok(m::ProposeSchemaChangesOutcome::Proposed {
            schema_changes_proposed: e,
        }) => Done {
            outcome: "proposed",
            ok: true,
            detail: detail(&e),
        },
        Ok(m::ProposeSchemaChangesOutcome::Applied {
            schema_changes_proposed: e,
        }) => Done {
            outcome: "applied",
            ok: true,
            detail: detail(&e),
        },
        Ok(m::ProposeSchemaChangesOutcome::SampleFailed { error }) => Done {
            outcome: "sample-failed",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        Ok(m::ProposeSchemaChangesOutcome::ProposeFailed { error }) => Done {
            outcome: "propose-failed",
            ok: false,
            detail: json!({"reason": error.reason}),
        },
        Ok(m::ProposeSchemaChangesOutcome::NoSuchInstance { error }) => Done {
            outcome: "no-such-instance",
            ok: false,
            detail: json!({"name": error.name.0}),
        },
    };
    print("schema", &done)
}

fn split(source_id: &str) -> (&str, &str) {
    source_id.split_once('/').unwrap_or((source_id, ""))
}

fn run_source(app: &mut App, ctx: &Ctx, source_id: &str, record: bool) -> ExitCode {
    let outcome = match app.run_source(m::RunSource {
        source_id: m::SourceId(source_id.to_string()),
    }) {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    let (done, failure) = match outcome {
        m::RunSourceOutcome::Ran { source_ran } => {
            let report = ctx.shared.borrow_mut().last_run.take().unwrap_or_default();
            let mut detail = json!({
                "source_id": source_ran.source_id.0,
                "documents_new": source_ran.documents_new,
                "documents_applied": source_ran.documents_applied,
                "cost_usd": source_ran.cost_usd.map(|c| c.0),
                "facts_refused": report.facts_refused,
                "parts_rejected": report.parts_rejected,
                "masked": report.masked,
                "stopped": report.stopped,
            });
            // Present only when a document was cut: a run that cut nothing reports as before.
            if report.truncated > 0 {
                detail["truncated"] = json!(report.truncated);
            }
            // Present only when a parent was skipped: a run that skipped none reports as before.
            if !report.skipped.is_empty() {
                detail["skipped"] = json!(report.skipped);
            }
            // Present only when EKR rejected a part: a run that had none reports as before.
            if !report.rejected.is_empty() {
                detail["rejected"] = json!(report
                    .rejected
                    .iter()
                    .map(run::Rejected::json)
                    .collect::<Vec<_>>());
            }
            // Present only when the policy pseudonymises: a spec file without one reports as
            // before.
            if let Some(redacted) = &report.redacted {
                detail["redacted"] = json!(redacted);
                detail["unrestored"] = json!(report.unrestored);
            }
            // Present only when the run ended a value (`dropped: Supersede`): a run that ended
            // none reports as before.
            if report.superseded + report.retracted > 0 {
                detail["superseded"] = json!(report.superseded);
                detail["retracted"] = json!(report.retracted);
            }
            (
                Done {
                    outcome: "ran",
                    ok: true,
                    detail,
                },
                None,
            )
        }
        m::RunSourceOutcome::FetchFailed { error } => (
            Done {
                outcome: "fetch-failed",
                ok: false,
                detail: json!({"reason": error.reason}),
            },
            Some(error.reason),
        ),
        m::RunSourceOutcome::ExtractionFailed { error } => (
            Done {
                outcome: "extraction-failed",
                ok: false,
                detail: json!({"reason": error.reason}),
            },
            Some(error.reason),
        ),
        m::RunSourceOutcome::ApplyRefused { error } => (
            Done {
                outcome: "apply-refused",
                ok: false,
                detail: json!({"reason": error.reason}),
            },
            Some(error.reason),
        ),
        m::RunSourceOutcome::Disabled { error } => (
            Done {
                outcome: "disabled",
                ok: false,
                detail: json!({"source_id": error.source_id.0}),
            },
            None,
        ),
        m::RunSourceOutcome::NoSuchSource { error } => (
            Done {
                outcome: "no-such-source",
                ok: false,
                detail: json!({"source_id": error.source_id.0}),
            },
            None,
        ),
    };
    let code = print("run", &done);
    if let (true, Some(reason)) = (record, failure) {
        record_failure(app, ctx, source_id, &reason);
    }
    code
}

fn record_failure(app: &mut App, ctx: &Ctx, source_id: &str, reason: &str) -> ExitCode {
    let done = match app.record_failure(m::RecordFailure {
        source_id: m::SourceId(source_id.to_string()),
        reason: reason.to_string(),
    }) {
        Err(e) => return fail(e),
        Ok(m::RecordFailureOutcome::Counted { run_failed }) => Done {
            outcome: "counted",
            ok: true,
            detail: json!({"source_id": run_failed.source_id.0}),
        },
        Ok(m::RecordFailureOutcome::Disabled { source_disabled }) => {
            let (instance, source) = split(source_id);
            let mut detail = json!({"source_id": source_disabled.source_id.0});
            if let Err(e) =
                Systemd::from_env(&ctx.home.root).set_source_timer(instance, source, false)
            {
                detail["units"] = json!({"failed": e});
            }
            Done {
                outcome: "disabled",
                ok: true,
                detail,
            }
        }
        Ok(m::RecordFailureOutcome::AlreadyDisabled { error }) => Done {
            outcome: "already-disabled",
            ok: false,
            detail: json!({"source_id": error.source_id.0}),
        },
        Ok(m::RecordFailureOutcome::NoSuchSource { error }) => Done {
            outcome: "no-such-source",
            ok: false,
            detail: json!({"source_id": error.source_id.0}),
        },
    };
    print("source record-failure", &done)
}

fn enable(app: &mut App, ctx: &Ctx, source_id: &str) -> ExitCode {
    let done = match app.enable_source(m::EnableSource {
        source_id: m::SourceId(source_id.to_string()),
    }) {
        Err(e) => return fail(e),
        Ok(m::EnableSourceOutcome::Enabled { source_enabled }) => {
            let (instance, source) = split(source_id);
            let mut detail = json!({"source_id": source_enabled.source_id.0});
            if let Err(e) =
                Systemd::from_env(&ctx.home.root).set_source_timer(instance, source, true)
            {
                detail["units"] = json!({"failed": e});
            }
            Done {
                outcome: "enabled",
                ok: true,
                detail,
            }
        }
        Ok(m::EnableSourceOutcome::WrongState { .. }) => Done {
            outcome: "wrong-state",
            ok: false,
            detail: json!({"source_id": source_id}),
        },
        Ok(m::EnableSourceOutcome::NoSuchSource { error }) => Done {
            outcome: "no-such-source",
            ok: false,
            detail: json!({"source_id": error.source_id.0}),
        },
    };
    print("source enable", &done)
}

fn mcp_line(home: &Home, name: &str) -> ExitCode {
    let layout = Layout::new(home.instance_dir(name));
    let spec = match layout.load_spec() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let store = layout.store_handle(&spec);
    println!(
        "claude mcp add --transport stdio cortex-{name} -- env EKR_HOST={} EKR_BACKEND={} EKR_STORE={} {} mcp",
        store.host.display(),
        store.backend.name(),
        store.store.display(),
        store.bin.display()
    );
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use clap::CommandFactory;
    use serde_json::Value;

    use super::Cli;

    /// Inputs the command line reads from the `--spec` file instead of a flag of their own.
    const FROM_SPEC_FILE: &[(&str, &[&str])] = &[
        (
            "cortex.instance.CreateInstance",
            &[
                "name",
                "description",
                "model",
                "ekr_version",
                "seed_digest",
                "spec",
            ],
        ),
        (
            "cortex.instance.UpdateInstance",
            &["description", "model", "spec", "seed_digest"],
        ),
        (
            "cortex.instance.AdoptInstance",
            &[
                "name",
                "description",
                "model",
                "ekr_version",
                "seed_digest",
                "spec",
            ],
        ),
    ];
    /// Flags that steer the implementation and are no command input.
    const STEERING: &[&str] = &[
        "spec",
        "host",
        "seen",
        "no_extract",
        "no_units",
        "replace_binary",
        "postgres_schema_config",
        "record_failure",
        "help",
    ];

    fn compiled() -> Value {
        let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
        let out = std::process::Command::new("ess")
            .args(["specify", "compile", "--path"])
            .arg(std::path::Path::new(&root).join("spec"))
            .args(["--format", "json"])
            .output()
            .expect("ess on PATH");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("compiled JSON")
    }

    fn find<'a>(v: &'a Value, name: &str) -> &'a Value {
        fn walk<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
            match v {
                Value::Object(o) => {
                    if o.get("name").and_then(Value::as_str) == Some(name)
                        && (o.contains_key("outcomes") || o.contains_key("consistency"))
                    {
                        return Some(v);
                    }
                    o.values().find_map(|c| walk(c, name))
                }
                Value::Array(a) => a.iter().find_map(|c| walk(c, name)),
                _ => None,
            }
        }
        walk(v, name).unwrap_or_else(|| panic!("{name} is not in the compiled model"))
    }

    fn check(command: &clap::Command, qualified: &str, model: &Value) {
        let decl = find(model, qualified);
        let wire = decl["naming"]["wire"].as_str().expect("a wire name");
        let sub = command
            .find_subcommand(wire)
            .unwrap_or_else(|| panic!("no `{wire}` subcommand for {qualified}"));
        let Some(inputs) = decl["input"].as_array() else {
            return;
        };
        let inputs: BTreeSet<&str> = inputs.iter().filter_map(|i| i["name"].as_str()).collect();
        let from_file: BTreeSet<&str> = FROM_SPEC_FILE
            .iter()
            .find(|(n, _)| *n == qualified)
            .map(|(_, f)| f.iter().copied().collect())
            .unwrap_or_default();
        let args: BTreeSet<&str> = sub
            .get_arguments()
            .filter(|a| !a.is_global_set())
            .map(|a| a.get_id().as_str())
            .collect();
        for input in &inputs {
            assert!(
                args.contains(input) || from_file.contains(input),
                "{qualified}: input `{input}` has no flag and is not read from the spec file"
            );
        }
        for arg in &args {
            assert!(
                inputs.contains(arg) || STEERING.contains(arg),
                "{qualified}: `{arg}` is neither an input nor a steering flag"
            );
        }
    }

    #[test]
    fn the_command_line_covers_every_command_and_view_the_specification_puts_on_it() {
        let model = compiled();
        let cli = &model["components"]["cortex"]["cli"];
        let root = Cli::command();
        for c in cli["commands"].as_array().expect("commands") {
            check(&root, c.as_str().unwrap(), &model);
        }
        for g in cli["groups"].as_array().expect("groups") {
            let group = root
                .find_subcommand(g["name"].as_str().unwrap())
                .expect("the group's subcommand");
            for c in g["commands"].as_array().expect("group commands") {
                check(group, c.as_str().unwrap(), &model);
            }
        }
        for v in cli["views"].as_array().expect("views") {
            check(&root, v.as_str().unwrap(), &model);
        }
    }
}

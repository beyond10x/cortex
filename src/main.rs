//! The `cortex` command line: clap derive over the generated behaviours of `spec/`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use clap::{Parser, Subcommand};
use cortex_model::behaviour::Generated;
use cortex_model::instance as m;
use cortex_model::ports::cortex::Cortex;
use serde_json::{json, Value};

use cortex_cli::connectors::Connectors;
use cortex_cli::home::Home;
use cortex_cli::instance::{Layout, Meta};
use cortex_cli::ports::{Ports, Shared, SharedRef};
use cortex_cli::schedule::Systemd;
use cortex_cli::{ekr, home, instance, model_map, run, spec};

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
    },
    /// Change an instance's sources, model or serve settings (`cortex.instance.UpdateInstance`).
    Update {
        name: String,
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        no_units: bool,
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
        #[arg(long, default_value = "0.0.30")]
        ekr_version: String,
    },
    /// Print the JSON Schema of the spec file (`cortex.instance.InstanceSpec`).
    Schema,
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
    match cli.command {
        Command::Schema => {
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
            let _lock = match home.lock() {
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
            };
            let result = dispatch(&mut app, &ctx, command);
            if let Err(e) = home.save_registry(&shared.borrow().registry) {
                return fail(e);
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
}

fn dispatch(app: &mut App, ctx: &Ctx, command: Command) -> ExitCode {
    match command {
        Command::Create {
            spec,
            no_extract,
            no_units,
        } => create(app, ctx, &spec, no_extract, no_units),
        Command::Update {
            name,
            spec,
            no_units,
        } => update(app, ctx, &name, &spec, no_units),
        Command::Remove { name } => remove(app, ctx, &name),
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
                            "view": port.map(|p| format!("http://127.0.0.1:{p}/")),
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
        Command::Schema | Command::Setup { .. } | Command::McpLine { .. } => {
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
fn seed_instance(layout: &Layout, loaded: &spec::Loaded) -> Result<(), String> {
    let spec = &loaded.model;
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
    store.seed(&seed_path)?;
    if let Some(schema) = &spec.seed.schema {
        let report = store.apply(&layout.dir.join(schema))?;
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

fn create(app: &mut App, ctx: &Ctx, path: &Path, no_extract: bool, no_units: bool) -> ExitCode {
    let loaded = match spec::load(path) {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let spec = loaded.model.clone();
    let name = spec.name.0.clone();
    let layout = Layout::new(ctx.home.instance_dir(&name));
    const CMD: &str = "cortex.instance.CreateInstance";

    let taken = ctx
        .shared
        .borrow_mut()
        .registry
        .instances
        .contains_key(&name);
    ctx.shared
        .borrow_mut()
        .external
        .insert((CMD, "name-taken"), taken);
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
                if layout.dir.exists() {
                    ctx.shared
                        .borrow_mut()
                        .external
                        .insert((CMD, "seed-refused"), true);
                    ctx.shared.borrow_mut().strings.push_back(format!(
                        "{} exists and is no registered instance",
                        layout.dir.display()
                    ));
                } else if let Err(reason) = seed_instance(&layout, &loaded) {
                    let _ = std::fs::remove_dir_all(&layout.dir);
                    ctx.shared
                        .borrow_mut()
                        .external
                        .insert((CMD, "seed-refused"), true);
                    ctx.shared.borrow_mut().strings.push_back(reason);
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
        m::CreateInstanceOutcome::Created { instance_created } => {
            let port = instance_created.view_port as u16;
            let meta = Meta {
                format: "cortex.instance-meta/1".into(),
                spec_dir: std::fs::canonicalize(&loaded.dir).unwrap_or(loaded.dir.clone()),
                view_port: port,
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
            if !no_extract && !spec.seed.documents.is_empty() {
                let tools = &ctx.tools;
                detail["seed"] = match run::seed(&layout, tools) {
                    Ok(r) => {
                        json!({"documents_applied": r.documents_applied, "cost_usd": r.cost_usd})
                    }
                    Err(e) => json!({"failed": format!("{e:?}")}),
                };
            }
            if !no_units {
                let systemd = Systemd::from_env(&ctx.home.root);
                let store = layout.store_handle(&spec);
                let installed = systemd
                    .install_view(&name, &store.bin, &store.host, &store.store, port)
                    .and_then(|_| {
                        spec.sources
                            .iter()
                            .try_for_each(|s| systemd.install_source(&name, &s.name, &s.schedule))
                    });
                if let Err(e) = installed {
                    detail["units"] = json!({"failed": e});
                }
            }
            Done {
                outcome: "created",
                ok: true,
                detail,
            }
        }
    };
    print("create", &done)
}

fn update(app: &mut App, ctx: &Ctx, name: &str, path: &Path, no_units: bool) -> ExitCode {
    let loaded = match spec::load(path) {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let spec = loaded.model.clone();
    if spec.name.0 != name {
        return fail(format!("the spec names {:?}, not {name:?}", spec.name.0));
    }
    let layout = Layout::new(ctx.home.instance_dir(name));
    let seed_changed = match layout.load_spec() {
        Ok(old) => {
            old.seed != spec.seed || seed_bytes(&layout.dir, &old) != seed_bytes(&loaded.dir, &spec)
        }
        Err(_) => false,
    };
    let input = m::UpdateInstance {
        name: spec.name.clone(),
        description: spec.description.clone(),
        model: spec.model.model.clone(),
        spec: spec.clone(),
        seed_changed,
    };
    let done = match app.update_instance(input) {
        Err(e) => return fail(e),
        Ok(m::UpdateInstanceOutcome::SeedChangeRefused { error }) => Done {
            outcome: "seed-change-refused",
            ok: false,
            detail: json!({"name": error.name.0}),
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
            if !no_units {
                let systemd = Systemd::from_env(&ctx.home.root);
                if let Err(e) = spec
                    .sources
                    .iter()
                    .try_for_each(|s| systemd.install_source(name, &s.name, &s.schedule))
                {
                    detail["units"] = json!({"failed": e});
                }
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

/// The bytes of every seed file a spec names, for comparing two specs' seeds.
fn seed_bytes(dir: &Path, spec: &m::InstanceSpec) -> Vec<(String, Vec<u8>)> {
    spec::seed_files(spec)
        .into_iter()
        .map(|rel| {
            let mut bytes = Vec::new();
            for entry in walkdir::WalkDir::new(dir.join(&rel))
                .sort_by_file_name()
                .into_iter()
                .flatten()
            {
                if entry.file_type().is_file() {
                    bytes.extend(std::fs::read(entry.path()).unwrap_or_default());
                }
            }
            (rel, bytes)
        })
        .collect()
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
            if let Err(e) = Systemd::from_env(&ctx.home.root).remove_instance(name, &sources) {
                detail["units"] = json!({"failed": e});
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
            (
                Done {
                    outcome: "ran",
                    ok: true,
                    detail: json!({
                        "source_id": source_ran.source_id.0,
                        "documents_new": source_ran.documents_new,
                        "documents_applied": source_ran.documents_applied,
                        "cost_usd": source_ran.cost_usd.map(|c| c.0),
                        "facts_refused": report.facts_refused,
                        "parts_rejected": report.parts_rejected,
                        "masked": report.masked,
                        "stopped": report.stopped,
                    }),
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
        "claude mcp add --transport stdio cortex-{name} -- env EKR_HOST={} EKR_BACKEND=sqlite EKR_STORE={} {} mcp",
        store.host.display(),
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
            &["name", "description", "model", "ekr_version", "spec"],
        ),
        (
            "cortex.instance.UpdateInstance",
            &["description", "model", "spec", "seed_changed"],
        ),
    ];
    /// Flags that steer the implementation and are no command input.
    const STEERING: &[&str] = &["spec", "no_extract", "no_units", "record_failure", "help"];

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

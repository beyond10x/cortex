//! Runs every scenario of `spec/suite.json` (written by `ess verify conform synthesize`) against
//! the generated behaviours over cortex's ports, in process. External branches are forced the way
//! each scenario says, through the context the ports ask first; `RunSource`'s pipeline is replaced
//! by a stand-in that answers `ran`. Every scenario must answer; an unknown step or value kind
//! fails rather than skips.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use cortex_cli::connectors::Connectors;
use cortex_cli::home::{self, Home, Registry};
use cortex_cli::ports::{Ports, Shared, SharedRef};
use cortex_cli::run::{Failure, Report, Tools};
use cortex_model::behaviour::Generated;
use cortex_model::instance as m;
use cortex_model::ports::cortex::{Cortex, PublishedEvent};
use serde_json::{json, Value};

type App = Cortex<Generated<Ports>>;

struct Last {
    input: Value,
    outcome: String,
    error: Option<String>,
    events: Vec<(String, Value)>,
}

struct Scenario {
    app: App,
    shared: SharedRef,
    captured: BTreeMap<String, Value>,
    observed: Vec<(String, Value)>,
    last: Option<Last>,
    views: BTreeMap<String, Vec<Value>>,
    snapshots: BTreeMap<String, Value>,
}

fn placeholder_spec() -> m::InstanceSpec {
    m::InstanceSpec {
        format: "cortex.instance/1".into(),
        name: m::InstanceName("placeholder".into()),
        description: String::new(),
        ekr: m::EkrPin {
            version: "0.0.30".into(),
            bin: None,
        },
        seed: m::SeedSpec {
            schema: None,
            ekr_seed: None,
            documents: vec![],
        },
        model: m::ModelSpec {
            model: "m".into(),
            budget_usd: cortex_model::primitives::Decimal("1".into()),
            timeout_s: 1,
            instructions: None,
        },
        sources: vec![],
        serve: m::ServeSpec { view_port: None },
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => text(a) == text(b),
    }
}

fn event(e: PublishedEvent) -> (String, Value) {
    let (name, payload) = match e {
        PublishedEvent::InstanceCreated(e) => (
            "InstanceCreated",
            json!({"name": e.name.0, "view_port": e.view_port}),
        ),
        PublishedEvent::InstanceRemoved(e) => ("InstanceRemoved", json!({"name": e.name.0})),
        PublishedEvent::InstanceUpdated(e) => ("InstanceUpdated", json!({"name": e.name.0})),
        PublishedEvent::RunFailed(e) => (
            "RunFailed",
            json!({"source_id": e.source_id.0, "reason": e.reason}),
        ),
        PublishedEvent::SourceAdded(e) => (
            "SourceAdded",
            json!({"source_id": e.source_id.0, "instance_name": e.instance_name.0}),
        ),
        PublishedEvent::SourceDisabled(e) => (
            "SourceDisabled",
            json!({"source_id": e.source_id.0, "reason": e.reason}),
        ),
        PublishedEvent::SourceEnabled(e) => ("SourceEnabled", json!({"source_id": e.source_id.0})),
        PublishedEvent::SourceRan(e) => (
            "SourceRan",
            json!({
                "source_id": e.source_id.0,
                "documents_new": e.documents_new,
                "documents_applied": e.documents_applied,
                "cost_usd": e.cost_usd.0,
            }),
        ),
    };
    (format!("cortex.instance.{name}"), payload)
}

impl Scenario {
    fn new() -> Self {
        let shared: SharedRef = Rc::new(RefCell::new(Shared {
            registry: Registry::default(),
            ..Shared::default()
        }));
        // External branches are forced through the context, which the ports ask first; a run
        // that reaches the pipeline is the `ran` branch.
        let runner =
            Box::new(|_: &m::SourceData| -> Result<Report, Failure> { Ok(Report::default()) });
        let app = Cortex::new(Generated::new(Ports {
            home: Home::new(std::env::temp_dir().join("cortex-conformance-unused")),
            tools: Tools {
                connectors: Connectors {
                    bin: "connectors-unused".into(),
                },
                claude: "claude-unused".into(),
            },
            shared: shared.clone(),
            runner: Some(runner),
        }));
        Self {
            app,
            shared,
            captured: BTreeMap::new(),
            observed: Vec::new(),
            last: None,
            views: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        }
    }

    fn resolve(&self, spec: &Value) -> Value {
        match spec["kind"].as_str() {
            Some("literal") => spec["value"].clone(),
            Some("instance") => self
                .captured
                .get(spec["instance"].as_str().unwrap())
                .cloned()
                .unwrap_or_else(|| panic!("nothing captured as {}", spec["instance"])),
            Some("observed") => self
                .observed
                .iter()
                .rev()
                .find(|(name, _)| name == spec["event"].as_str().unwrap())
                .map(|(_, payload)| payload[spec["field"].as_str().unwrap()].clone())
                .unwrap_or_else(|| panic!("no {} observed", spec["event"])),
            other => panic!("unsupported value kind {other:?}"),
        }
    }

    fn execute(&mut self, step: &Value) {
        let command = step["command"].as_str().unwrap();
        let input: serde_json::Map<String, Value> = step["input"]
            .as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect()
            })
            .unwrap_or_default();
        let s = |k: &str| text(input.get(k).unwrap_or(&Value::Null));
        let app = &mut self.app;
        let (outcome, error): (&str, Option<&str>) = match command {
            "cortex.instance.CreateInstance" => match app
                .create_instance(m::CreateInstance {
                    name: m::InstanceName(s("name")),
                    description: s("description"),
                    model: s("model"),
                    ekr_version: s("ekr_version"),
                    spec: placeholder_spec(),
                })
                .unwrap()
            {
                m::CreateInstanceOutcome::NameTaken { .. } => ("name-taken", Some("NameTaken")),
                m::CreateInstanceOutcome::ConnectionMissing { .. } => {
                    ("connection-missing", Some("ConnectionMissing"))
                }
                m::CreateInstanceOutcome::SeedRefused { .. } => {
                    ("seed-refused", Some("SeedRefused"))
                }
                m::CreateInstanceOutcome::Created { .. } => ("created", None),
            },
            "cortex.instance.UpdateInstance" => match app
                .update_instance(m::UpdateInstance {
                    name: m::InstanceName(s("name")),
                    description: s("description"),
                    model: s("model"),
                    spec: placeholder_spec(),
                    seed_changed: input
                        .get("seed_changed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
                .unwrap()
            {
                m::UpdateInstanceOutcome::SeedChangeRefused { .. } => {
                    ("seed-change-refused", Some("SeedChangeRefused"))
                }
                m::UpdateInstanceOutcome::Updated { .. } => ("updated", None),
                m::UpdateInstanceOutcome::NotActive { .. } => {
                    ("not-active", Some("InstanceNotActive"))
                }
                m::UpdateInstanceOutcome::NoSuchInstance { .. } => {
                    ("no-such-instance", Some("InstanceNotFound"))
                }
            },
            "cortex.instance.RemoveInstance" => match app
                .remove_instance(m::RemoveInstance {
                    name: m::InstanceName(s("name")),
                })
                .unwrap()
            {
                m::RemoveInstanceOutcome::Removed { .. } => ("removed", None),
                m::RemoveInstanceOutcome::WrongState { .. } => {
                    ("wrong-state", Some("InstanceNotActive"))
                }
                m::RemoveInstanceOutcome::NoSuchInstance { .. } => {
                    ("no-such-instance", Some("InstanceNotFound"))
                }
            },
            "cortex.instance.AddSource" => match app
                .add_source(m::AddSource {
                    instance_name: m::InstanceName(s("instance_name")),
                    source_id: m::SourceId(s("source_id")),
                    name: s("name"),
                    kind: match s("kind").as_str() {
                        "Web" => m::SourceKind::Web,
                        "Connectors" => m::SourceKind::Connectors,
                        _ => m::SourceKind::Files,
                    },
                    schedule: s("schedule"),
                })
                .unwrap()
            {
                m::AddSourceOutcome::Added { .. } => ("added", None),
            },
            "cortex.instance.RunSource" => match app
                .run_source(m::RunSource {
                    source_id: m::SourceId(s("source_id")),
                })
                .unwrap()
            {
                m::RunSourceOutcome::FetchFailed { .. } => ("fetch-failed", Some("FetchFailed")),
                m::RunSourceOutcome::ExtractionFailed { .. } => {
                    ("extraction-failed", Some("ExtractionFailed"))
                }
                m::RunSourceOutcome::ApplyRefused { .. } => ("apply-refused", Some("ApplyRefused")),
                m::RunSourceOutcome::Ran { .. } => ("ran", None),
                m::RunSourceOutcome::Disabled { .. } => ("disabled", Some("SourceDisabledError")),
                m::RunSourceOutcome::NoSuchSource { .. } => {
                    ("no-such-source", Some("SourceNotFound"))
                }
            },
            "cortex.instance.RecordFailure" => match app
                .record_failure(m::RecordFailure {
                    source_id: m::SourceId(s("source_id")),
                    reason: s("reason"),
                })
                .unwrap()
            {
                m::RecordFailureOutcome::Disabled { .. } => ("disabled", None),
                m::RecordFailureOutcome::Counted { .. } => ("counted", None),
                m::RecordFailureOutcome::AlreadyDisabled { .. } => {
                    ("already-disabled", Some("SourceDisabledError"))
                }
                m::RecordFailureOutcome::NoSuchSource { .. } => {
                    ("no-such-source", Some("SourceNotFound"))
                }
            },
            "cortex.instance.EnableSource" => match app
                .enable_source(m::EnableSource {
                    source_id: m::SourceId(s("source_id")),
                })
                .unwrap()
            {
                m::EnableSourceOutcome::Enabled { .. } => ("enabled", None),
                m::EnableSourceOutcome::WrongState { .. } => {
                    ("wrong-state", Some("SourceNotDisabled"))
                }
                m::EnableSourceOutcome::NoSuchSource { .. } => {
                    ("no-such-source", Some("SourceNotFound"))
                }
            },
            other => panic!("unknown command {other}"),
        };
        let events: Vec<_> = self.app.drain_outbox().into_iter().map(event).collect();
        self.observed.extend(events.iter().cloned());
        // A forced branch holds for the one command that follows it.
        self.shared.borrow_mut().external.clear();
        self.last = Some(Last {
            input: Value::Object(input),
            outcome: outcome.to_string(),
            error: error.map(|e| format!("cortex.instance.{e}")),
            events,
        });
    }

    fn rows(&self, view: &str) -> Vec<Value> {
        let shared = self.shared.borrow();
        match view {
            "cortex.instance.Instances" => shared
                .registry
                .instances
                .values()
                .map(|i| {
                    json!({
                        "name": i.data.name.0,
                        "description": i.data.description,
                        "model": i.data.model,
                        "ekr_version": i.data.ekr_version,
                        "state": home::instance_state_name(i.state),
                    })
                })
                .collect(),
            "cortex.instance.Sources" => shared
                .registry
                .sources
                .values()
                .map(|s| {
                    json!({
                        "source_id": s.data.source_id.0,
                        "instance_name": s.data.instance_name.0,
                        "name": s.data.name,
                        "kind": home::kind_name(s.data.kind),
                        "schedule": s.data.schedule,
                        "runs": s.data.runs,
                        "consecutive_failures": s.data.consecutive_failures,
                        "state": home::source_state_name(s.state),
                    })
                })
                .collect(),
            other => panic!("unknown view {other}"),
        }
    }

    fn subject_row(&self, step: &Value) -> Option<Value> {
        let id_field = step["shape"]["identity_field"].as_str().unwrap();
        let want = self.resolve(&step["subject"][id_field]);
        self.rows(step["view"].as_str().unwrap())
            .into_iter()
            .find(|r| same(&r[id_field], &want))
    }

    fn step(&mut self, step: &Value) {
        let last = || self.last.as_ref().expect("a command ran");
        match step["step"].as_str().unwrap() {
            "configure_external_outcome" => {
                let command = step["force"]["command"].as_str().unwrap().to_string();
                let outcome = step["force"]["outcome"].as_str().unwrap().to_string();
                let key: (&'static str, &'static str) = (
                    Box::leak(command.into_boxed_str()),
                    Box::leak(outcome.into_boxed_str()),
                );
                self.shared.borrow_mut().external.insert(key, true);
            }
            "execute_command" => self.execute(step),
            "expect_outcome" => {
                let want = step["outcome"]["outcome"].as_str().unwrap();
                assert_eq!(last().outcome, want);
            }
            "expect_error" => {
                assert_eq!(last().error.as_deref(), step["error"].as_str());
            }
            "expect_no_error" => assert_eq!(last().error, None),
            "expect_event" => {
                let name = step["event"].as_str().unwrap();
                let (_, payload) = last()
                    .events
                    .iter()
                    .find(|(n, _)| n == name)
                    .unwrap_or_else(|| panic!("{name} was not published"));
                if let Some(fields) = step["payload"].as_object() {
                    for (field, input_field) in fields {
                        let want = &last().input[input_field.as_str().unwrap()];
                        assert!(
                            same(&payload[field], want),
                            "{name}.{field}: {} != {want}",
                            payload[field]
                        );
                    }
                }
                if let Some(shape) = step["shape"].as_object() {
                    for field in shape.keys() {
                        assert!(payload.get(field).is_some(), "{name} lacks {field}");
                    }
                }
            }
            "expect_no_event" => {
                let name = step["event"].as_str().unwrap();
                assert!(
                    !last().events.iter().any(|(n, _)| n == name),
                    "{name} was published"
                );
            }
            "expect_no_events" => assert!(last().events.is_empty(), "events were published"),
            "capture_instance" => {
                let event = step["event"].as_str().unwrap();
                let field = step["field"].as_str().unwrap();
                let (_, payload) = last()
                    .events
                    .iter()
                    .find(|(n, _)| n == event)
                    .unwrap_or_else(|| panic!("{event} was not published"));
                let value = payload[field].clone();
                self.captured
                    .insert(step["instance"].as_str().unwrap().to_string(), value);
            }
            "query_view" => {
                let view = step["view"].as_str().unwrap();
                let rows = self.rows(view);
                self.views.insert(view.to_string(), rows);
            }
            "expect_view" => {
                let view = step["view"].as_str().unwrap();
                let rows = self.views.get(view).expect("the view was queried");
                let expectation = &step["expectation"];
                match expectation["expect"].as_str().unwrap() {
                    "contains" => {
                        let fields: Vec<(String, Value)> = expectation["fields"]
                            .as_object()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| (k.clone(), self.resolve(v)))
                            .collect();
                        assert!(
                            rows.iter()
                                .any(|r| fields.iter().all(|(k, v)| same(&r[k], v))),
                            "{view} holds no row with {fields:?}: {rows:?}"
                        );
                    }
                    "satisfies" => {
                        let p = expectation["predicate"].as_str().unwrap();
                        let parts: Vec<&str> = p.split_whitespace().collect();
                        let [field, op, bound] = parts[..] else {
                            panic!("predicate {p:?}")
                        };
                        let bound: f64 = bound.parse().unwrap();
                        for r in rows {
                            let v = r[field].as_f64().unwrap();
                            let ok = match op {
                                ">=" => v >= bound,
                                ">" => v > bound,
                                "<=" => v <= bound,
                                "<" => v < bound,
                                "==" => v == bound,
                                other => panic!("operator {other}"),
                            };
                            assert!(ok, "{view}: {p} fails for {r}");
                        }
                    }
                    other => panic!("unsupported expectation {other}"),
                }
            }
            "snapshot_complete_subject" => {
                let row = self.subject_row(step);
                self.snapshots.insert(
                    step["view"].as_str().unwrap().to_string(),
                    row.unwrap_or(Value::Null),
                );
            }
            "expect_complete_subject_unchanged" => {
                let view = step["view"].as_str().unwrap();
                let before = self.snapshots.get(view).expect("a snapshot").clone();
                let id_field = before
                    .as_object()
                    .and_then(|o| {
                        ["name", "source_id"]
                            .into_iter()
                            .find(|k| o.contains_key(*k))
                    })
                    .expect("an identity field");
                let after = self
                    .rows(view)
                    .into_iter()
                    .find(|r| same(&r[id_field], &before[id_field]))
                    .unwrap_or(Value::Null);
                assert_eq!(before, after, "{view} changed");
            }
            other => panic!("unsupported step {other}"),
        }
    }
}

#[test]
fn every_scenario_of_the_synthesized_suite_holds() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let suite: Value = serde_json::from_slice(
        &std::fs::read(std::path::Path::new(&root).join("spec/suite.json")).unwrap(),
    )
    .unwrap();
    let scenarios = suite["scenarios"].as_object().expect("scenarios");
    let mut failed = Vec::new();
    for (name, scenario) in scenarios {
        let steps = scenario["steps"].as_array().expect("steps").clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = Scenario::new();
            for step in &steps {
                s.step(step);
            }
        }));
        if let Err(e) = result {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            failed.push(format!("{name}: {msg}"));
        }
    }
    assert_eq!(
        scenarios.len(),
        35,
        "the suite's scenario count moved; update this floor"
    );
    assert!(
        failed.is_empty(),
        "{} of {} scenarios failed:\n{}",
        failed.len(),
        scenarios.len(),
        failed.join("\n")
    );
}

//! The ports the generated behaviours run over: the registry as storage, the context that answers
//! each `external:` branch and each `{generated: true}` value, and the two obligations
//! `generated/cortex-model/PLAN.md` leaves to this crate, `RunSource` and `RecordFailure`.
//!
//! The generated `Cortex<B>` keeps its behaviours private, so the state the command line also
//! reads and writes lives in [`Shared`], held by both.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use cortex_model::behaviour::{Context, InstanceStorage, SourceStorage};
use cortex_model::instance as m;
use cortex_model::instance::obligations::{RecordFailureBehavior, RunSourceBehavior};
use cortex_model::obligation::UnmetObligation;
use cortex_model::primitives::Decimal;

use crate::home::{Home, Registry};
use crate::instance::Layout;
use crate::run::{self, Failure, Report, Tools};

#[derive(Default)]
pub struct Shared {
    pub registry: Registry,
    /// The `external:` branches the caller has already decided, by (command, outcome).
    pub external: BTreeMap<(&'static str, &'static str), bool>,
    /// Values for `{generated: true}` fields, taken in the order the behaviour asks.
    pub strings: VecDeque<String>,
    pub integers: VecDeque<i64>,
    /// The report of the last run, for the caller to print.
    pub last_run: Option<Report>,
}

pub type SharedRef = Rc<RefCell<Shared>>;

/// A stand-in for the run pipeline, which the conformance runner injects to force each declared
/// outcome of `RunSource` without fetching anything.
pub type Runner = Box<dyn FnMut(&m::SourceData) -> Result<Report, Failure>>;

pub struct Ports {
    pub home: Home,
    pub tools: Tools,
    pub shared: SharedRef,
    /// `None` runs the real pipeline (`run::run`).
    pub runner: Option<Runner>,
}

impl InstanceStorage for Ports {
    fn get(&self, identity: &m::InstanceName) -> Option<m::InstanceSnapshot> {
        self.shared
            .borrow()
            .registry
            .instances
            .get(&identity.0)
            .cloned()
    }
    fn put(&mut self, snapshot: m::InstanceSnapshot) {
        self.shared
            .borrow_mut()
            .registry
            .instances
            .insert(snapshot.data.name.0.clone(), snapshot);
    }
    fn delete(&mut self, identity: &m::InstanceName) {
        self.shared
            .borrow_mut()
            .registry
            .instances
            .remove(&identity.0);
    }
    fn list(&self) -> Vec<m::InstanceSnapshot> {
        self.shared
            .borrow()
            .registry
            .instances
            .values()
            .cloned()
            .collect()
    }
}

impl SourceStorage for Ports {
    fn get(&self, identity: &m::SourceId) -> Option<m::SourceSnapshot> {
        self.shared
            .borrow()
            .registry
            .sources
            .get(&identity.0)
            .cloned()
    }
    fn put(&mut self, snapshot: m::SourceSnapshot) {
        self.shared
            .borrow_mut()
            .registry
            .sources
            .insert(snapshot.data.source_id.0.clone(), snapshot);
    }
    fn delete(&mut self, identity: &m::SourceId) {
        self.shared
            .borrow_mut()
            .registry
            .sources
            .remove(&identity.0);
    }
    fn list(&self) -> Vec<m::SourceSnapshot> {
        self.shared
            .borrow()
            .registry
            .sources
            .values()
            .cloned()
            .collect()
    }
}

impl Context for Ports {
    /// Only `InstanceNotActive` asks, and it is answered only from `Removed`.
    fn generate_cortex_instance_instance_state(&mut self) -> m::InstanceState {
        m::InstanceState::Removed
    }
    /// Only `SourceNotDisabled` asks, and it is answered only from `Enabled`.
    fn generate_cortex_instance_source_state(&mut self) -> m::SourceState {
        m::SourceState::Enabled
    }
    fn generate_integer(&mut self) -> i64 {
        self.shared.borrow_mut().integers.pop_front().unwrap_or(0)
    }
    fn generate_string(&mut self) -> String {
        self.shared
            .borrow_mut()
            .strings
            .pop_front()
            .unwrap_or_default()
    }
    fn external(&mut self, command: &'static str, outcome: &'static str) -> bool {
        self.shared
            .borrow()
            .external
            .get(&(command, outcome))
            .copied()
            .unwrap_or(false)
    }
}

fn store(ports: &mut Ports, data: m::SourceData, state: m::SourceState) {
    let source = m::Source::new(data);
    let snapshot = match state {
        m::SourceState::Enabled => m::AnySource::Enabled(source),
        m::SourceState::Disabled => m::AnySource::Disabled(source.disable()),
    }
    .snapshot();
    SourceStorage::put(ports, snapshot);
}

impl RecordFailureBehavior for Ports {
    fn record_failure(
        &mut self,
        input: m::RecordFailure,
    ) -> Result<m::RecordFailureOutcome, UnmetObligation> {
        let Some(found) = SourceStorage::get(self, &input.source_id) else {
            return Ok(m::RecordFailureOutcome::NoSuchSource {
                error: m::SourceNotFound {
                    source_id: input.source_id,
                },
            });
        };
        if found.state == m::SourceState::Disabled {
            return Ok(m::RecordFailureOutcome::AlreadyDisabled {
                error: m::SourceDisabledError {
                    source_id: input.source_id,
                },
            });
        }
        let mut data = found.data;
        let disable = data.consecutive_failures >= 1;
        data.consecutive_failures += 1;
        if disable {
            store(self, data, m::SourceState::Disabled);
            Ok(m::RecordFailureOutcome::Disabled {
                source_disabled: m::SourceDisabled {
                    source_id: input.source_id,
                    reason: input.reason,
                },
            })
        } else {
            store(self, data, m::SourceState::Enabled);
            Ok(m::RecordFailureOutcome::Counted {
                run_failed: m::RunFailed {
                    source_id: input.source_id,
                    reason: input.reason,
                },
            })
        }
    }
}

impl RunSourceBehavior for Ports {
    fn run_source(&mut self, input: m::RunSource) -> Result<m::RunSourceOutcome, UnmetObligation> {
        // As the generated behaviours do: an external branch the context has already decided is
        // taken first, in declaration order. A real run decides none in advance; the pipeline
        // below decides them.
        const RUN: &str = "cortex.instance.RunSource";
        if self.external(RUN, "fetch-failed") {
            let reason = self.generate_string();
            return Ok(m::RunSourceOutcome::FetchFailed {
                error: m::FetchFailed { reason },
            });
        }
        if self.external(RUN, "extraction-failed") {
            let reason = self.generate_string();
            return Ok(m::RunSourceOutcome::ExtractionFailed {
                error: m::ExtractionFailed { reason },
            });
        }
        if self.external(RUN, "apply-refused") {
            let reason = self.generate_string();
            return Ok(m::RunSourceOutcome::ApplyRefused {
                error: m::ApplyRefused { reason },
            });
        }
        let Some(found) = SourceStorage::get(self, &input.source_id) else {
            return Ok(m::RunSourceOutcome::NoSuchSource {
                error: m::SourceNotFound {
                    source_id: input.source_id,
                },
            });
        };
        if found.state == m::SourceState::Disabled {
            return Ok(m::RunSourceOutcome::Disabled {
                error: m::SourceDisabledError {
                    source_id: input.source_id,
                },
            });
        }
        let layout = Layout::new(self.home.instance_dir(&found.data.instance_name.0));
        let result = match self.runner.as_mut() {
            Some(runner) => runner(&found.data),
            None => run::run(&layout, &self.tools, &found.data),
        };
        let report = match result {
            Err(Failure::Fetch(reason)) => {
                return Ok(m::RunSourceOutcome::FetchFailed {
                    error: m::FetchFailed { reason },
                })
            }
            Err(Failure::Extract(reason)) => {
                return Ok(m::RunSourceOutcome::ExtractionFailed {
                    error: m::ExtractionFailed { reason },
                })
            }
            Err(Failure::Apply(reason)) => {
                return Ok(m::RunSourceOutcome::ApplyRefused {
                    error: m::ApplyRefused { reason },
                })
            }
            Ok(report) => report,
        };
        let mut data = found.data;
        data.runs += 1;
        data.consecutive_failures = 0;
        store(self, data, m::SourceState::Enabled);
        let ran = m::SourceRan {
            source_id: input.source_id,
            documents_new: report.documents_new,
            documents_applied: report.documents_applied,
            cost_usd: report.cost_usd.map(|c| Decimal(format!("{c:.4}"))),
        };
        self.shared.borrow_mut().last_run = Some(report);
        Ok(m::RunSourceOutcome::Ran { source_ran: ran })
    }
}

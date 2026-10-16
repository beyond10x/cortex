//! The ports the generated behaviours run over: the registry as storage, the context that answers
//! each `external:` branch and each `{generated: true}` value, and the two obligations
//! `generated/cortex-model/PLAN.md` leaves to this crate, `RunSource` and `RecordFailure`.
//!
//! The generated `Cortex<B>` keeps its behaviours private, so the state the command line also
//! reads and writes lives in [`Shared`], held by both.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use cortex_model::behaviour::{Context, ExternalCommand, InstanceStorage, SourceStorage};
use cortex_model::instance as m;
use cortex_model::instance::obligations::{RecordFailureBehavior, RunSourceBehavior};
use cortex_model::obligation::UnmetObligation;
use cortex_model::primitives::{Decimal, Timestamp};

use crate::home::{Home, Registry};
use crate::instance::Layout;
use crate::run::{self, Counts, Failure, Report, Tools};

#[derive(Default)]
pub struct Shared {
    pub registry: Registry,
    /// The `external:` branches the caller has already decided, by (command, outcome).
    pub external: BTreeMap<(&'static str, &'static str), bool>,
    /// Values for `{generated: true}` fields, taken in the order the behaviour asks.
    pub strings: VecDeque<String>,
    pub integers: VecDeque<i64>,
    pub decimals: VecDeque<Decimal>,
    /// Values for `Optional<Decimal>` fields: `None` where there is nothing to report.
    pub optional_decimals: VecDeque<Option<Decimal>>,
    /// The report of the last run, for the caller to print.
    pub last_run: Option<Report>,
    /// When the last `RunSource` started, in Unix milliseconds, once it recorded an outcome.
    pub run_at_ms: Option<i64>,
}

pub type SharedRef = Rc<RefCell<Shared>>;

/// A stand-in for the run pipeline, which the conformance runner injects to force each declared
/// outcome of `RunSource` without fetching anything.
pub type Runner = Box<dyn FnMut(&m::SourceData) -> Result<Report, Failure>>;

/// A stand-in for the fetch-only pipeline (`run::count`), which the conformance runner injects.
pub type Counter = Box<dyn FnMut(&m::SourceData) -> Result<Counts, Failure>>;

pub struct Ports {
    pub home: Home,
    pub tools: Tools,
    pub shared: SharedRef,
    /// `None` runs the real pipeline (`run::run`).
    pub runner: Option<Runner>,
    /// `None` counts with the real pipeline (`run::count`).
    pub counter: Option<Counter>,
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
    fn generate_decimal(&mut self) -> Decimal {
        self.shared
            .borrow_mut()
            .decimals
            .pop_front()
            .unwrap_or_else(|| Decimal("0".into()))
    }
    /// Nothing to report when the caller queued no value.
    fn generate_optional_decimal(&mut self) -> Option<Decimal> {
        self.shared
            .borrow_mut()
            .optional_decimals
            .pop_front()
            .flatten()
    }
    fn generate_string(&mut self) -> String {
        self.shared
            .borrow_mut()
            .strings
            .pop_front()
            .unwrap_or_default()
    }
    fn external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> bool {
        self.decided(command.name(), outcome)
    }
}

/// A cost in USD as an event carries it: four places, `None` when an answer carried no cost.
pub fn cost_decimal(cost_usd: Option<f64>) -> Option<Decimal> {
    cost_usd.map(|c| Decimal(format!("{c:.4}")))
}

/// The `SourceRan` event of a run of `source_id` that ran.
fn source_ran(source_id: m::SourceId, report: &Report) -> m::SourceRan {
    m::SourceRan {
        source_id,
        documents_new: report.documents_new,
        documents_applied: report.documents_applied,
        cost_usd: cost_decimal(report.cost_usd),
        daily_ceiling_reached: report.daily_ceiling_reached,
        model_calls: report.model_calls,
        input_tokens: report.usage.input_tokens,
        cache_read_input_tokens: report.usage.cache_read_input_tokens,
        cache_creation_input_tokens: report.usage.cache_creation_input_tokens,
        output_tokens: report.usage.output_tokens,
        fixed_input_tokens: report.fixed_input_tokens,
        documents_input_tokens: report.documents_input_tokens,
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

impl Ports {
    /// Whether the caller has already decided the `external:` branch `outcome` of `command`, by
    /// the command's qualified name. `RunSource` is an obligation, not a generated behaviour, so
    /// it has no [`ExternalCommand`] and asks here directly.
    fn decided(&self, command: &'static str, outcome: &'static str) -> bool {
        self.shared
            .borrow()
            .external
            .get(&(command, outcome))
            .copied()
            .unwrap_or(false)
    }

    /// The active instance `<instance>/seed` names, when the instance has no source of that name:
    /// its seed documents, which `create` extracts as the pseudo-source `seed`.
    fn seed_of(&self, source_id: &m::SourceId) -> Option<m::InstanceName> {
        let (instance, source) = source_id.0.split_once('/')?;
        let name = m::InstanceName(instance.to_string());
        let held = InstanceStorage::get(self, &name)?;
        (source == SEED && held.state == m::InstanceState::Active).then_some(name)
    }

    /// Extracts the instance's seed documents no earlier seed extraction applied: those a seed that
    /// stopped early left, and none once every one is applied. Answered as a run of a source is,
    /// with no source to count the run against; kept as a snapshot and held to the gate as a
    /// source run is ([`run::rerun_seed`]). An adopted instance is refused as `fetch-failed`.
    fn run_seed(
        &mut self,
        instance: &m::InstanceName,
        source_id: m::SourceId,
    ) -> m::RunSourceOutcome {
        let layout = Layout::new(self.home.instance_dir(&instance.0));
        if layout.load_meta().is_ok_and(|meta| meta.adopted) {
            return m::RunSourceOutcome::FetchFailed {
                error: m::FetchFailed {
                    reason: format!(
                        "{} was adopted from an existing store: its seed documents belong to the \
                         store's earlier life, so `run {}/seed` extracts nothing into it",
                        instance.0, instance.0
                    ),
                },
            };
        }
        match run::rerun_seed(&layout, &self.tools) {
            Err(Failure::Fetch(reason)) => m::RunSourceOutcome::FetchFailed {
                error: m::FetchFailed { reason },
            },
            Err(Failure::Extract(reason)) => m::RunSourceOutcome::ExtractionFailed {
                error: m::ExtractionFailed { reason },
            },
            Err(Failure::Apply(reason)) => m::RunSourceOutcome::ApplyRefused {
                error: m::ApplyRefused { reason },
            },
            Err(Failure::StoreOutcomeUnknown(reason)) => m::RunSourceOutcome::StoreOutcomeUnknown {
                error: m::StoreOutcomeUnknown { reason },
            },
            Ok(report) => {
                let ran = source_ran(source_id, &report);
                self.shared.borrow_mut().last_run = Some(report);
                m::RunSourceOutcome::Ran { source_ran: ran }
            }
        }
    }
}

/// The name `create` records the seed documents under, as a source of the instance.
const SEED: &str = "seed";

impl Ports {
    /// Records the run's outcome on the source: `last_outcome`, `last_run_at` (when the run
    /// started) and `last_reason`, in the state the source holds.
    fn note(
        &mut self,
        found: m::SourceSnapshot,
        outcome: m::RunOutcome,
        reason: Option<String>,
    ) -> m::SourceData {
        let at = self.run_at();
        let mut data = found.data;
        data.last_outcome = Some(outcome);
        data.last_run_at = Some(Timestamp(crate::sources::rfc3339(at)));
        data.last_reason = reason;
        store(self, data.clone(), found.state);
        data
    }

    /// When the run started, in Unix milliseconds: taken once per run and kept in [`Shared`].
    fn run_at(&mut self) -> i64 {
        *self
            .shared
            .borrow_mut()
            .run_at_ms
            .get_or_insert_with(crate::instance::now_ms)
    }

    /// `fetch_only`: fetches the source and counts what it fetched against its seen state, in
    /// either state the source is in, and records nothing on it. A failure of any kind answers
    /// `fetch-only-failed`; `<instance>/seed` names no source and answers it too.
    fn fetch_only(&mut self, source_id: m::SourceId) -> m::RunSourceOutcome {
        const RUN: &str = "cortex.instance.RunSource";
        let failed = |reason| m::RunSourceOutcome::FetchOnlyFailed {
            error: m::FetchFailed { reason },
        };
        let Some(found) = SourceStorage::get(self, &source_id) else {
            if self.seed_of(&source_id).is_some() {
                return failed(format!(
                    "{} names the instance's seed documents, not a source: --fetch-only counts a \
                     source's documents",
                    source_id.0
                ));
            }
            return m::RunSourceOutcome::NoSuchSource {
                error: m::SourceNotFound { source_id },
            };
        };
        if self.decided(RUN, "fetch-only-failed") {
            return failed(self.generate_string());
        }
        let layout = Layout::new(self.home.instance_dir(&found.data.instance_name.0));
        let counted = match self.counter.as_mut() {
            Some(counter) => counter(&found.data),
            None => run::count(&layout, &self.tools, &found.data),
        };
        match counted {
            Err(
                Failure::Fetch(reason)
                | Failure::Extract(reason)
                | Failure::Apply(reason)
                | Failure::StoreOutcomeUnknown(reason),
            ) => failed(reason),
            Ok(c) => m::RunSourceOutcome::Fetched {
                source_fetched: m::SourceFetched {
                    source_id,
                    since: Timestamp(crate::sources::rfc3339(c.since_ms)),
                    until: Timestamp(crate::sources::rfc3339(c.until_ms)),
                    fetched: c.fetched as i64,
                    new: c.new as i64,
                    changed: c.changed as i64,
                    held: c.held as i64,
                    unchanged: c.unchanged as i64,
                    unread: c.unread,
                },
            },
        }
    }
}

impl RunSourceBehavior for Ports {
    fn run_source(&mut self, input: m::RunSource) -> Result<m::RunSourceOutcome, UnmetObligation> {
        // As the generated behaviours do: the source is read first, its held state answers
        // `disabled`, then an external branch the context has already decided is taken, in
        // declaration order. A real run decides none in advance; the pipeline below decides
        // them. Every outcome that names a source records itself on it.
        const RUN: &str = "cortex.instance.RunSource";
        self.shared.borrow_mut().run_at_ms = None;
        if input.fetch_only == Some(true) {
            return Ok(self.fetch_only(input.source_id));
        }
        let Some(found) = SourceStorage::get(self, &input.source_id) else {
            if let Some(instance) = self.seed_of(&input.source_id) {
                return Ok(self.run_seed(&instance, input.source_id));
            }
            return Ok(m::RunSourceOutcome::NoSuchSource {
                error: m::SourceNotFound {
                    source_id: input.source_id,
                },
            });
        };
        if found.state == m::SourceState::Disabled {
            self.note(found, m::RunOutcome::Disabled, None);
            return Ok(m::RunSourceOutcome::Disabled {
                run_skipped: m::RunSkipped {
                    source_id: input.source_id,
                },
            });
        }
        let forced = [
            ("fetch-failed", Failure::Fetch as fn(String) -> Failure),
            ("extraction-failed", Failure::Extract),
            ("apply-refused", Failure::Apply),
            ("store-outcome-unknown", Failure::StoreOutcomeUnknown),
        ]
        .into_iter()
        .find(|(outcome, _)| self.decided(RUN, outcome));
        let result = match forced {
            Some((_, failure)) => Err(failure(self.generate_string())),
            None => {
                self.run_at();
                let layout = Layout::new(self.home.instance_dir(&found.data.instance_name.0));
                match self.runner.as_mut() {
                    Some(runner) => runner(&found.data),
                    None => run::run(&layout, &self.tools, &found.data),
                }
            }
        };
        let report = match result {
            Err(Failure::Fetch(reason)) => {
                self.note(found, m::RunOutcome::FetchFailed, Some(reason.clone()));
                return Ok(m::RunSourceOutcome::FetchFailed {
                    error: m::FetchFailed { reason },
                });
            }
            Err(Failure::Extract(reason)) => {
                self.note(found, m::RunOutcome::ExtractionFailed, Some(reason.clone()));
                return Ok(m::RunSourceOutcome::ExtractionFailed {
                    error: m::ExtractionFailed { reason },
                });
            }
            Err(Failure::Apply(reason)) => {
                self.note(found, m::RunOutcome::ApplyRefused, Some(reason.clone()));
                return Ok(m::RunSourceOutcome::ApplyRefused {
                    error: m::ApplyRefused { reason },
                });
            }
            // Recorded on the source like a failure; the scheduler counts none for it (main.rs).
            Err(Failure::StoreOutcomeUnknown(reason)) => {
                self.note(
                    found,
                    m::RunOutcome::StoreOutcomeUnknown,
                    Some(reason.clone()),
                );
                return Ok(m::RunSourceOutcome::StoreOutcomeUnknown {
                    error: m::StoreOutcomeUnknown { reason },
                });
            }
            Ok(report) => report,
        };
        let mut data = self.note(found, m::RunOutcome::Ran, None);
        data.runs += 1;
        data.consecutive_failures = 0;
        store(self, data, m::SourceState::Enabled);
        let ran = source_ran(input.source_id, &report);
        self.shared.borrow_mut().last_run = Some(report);
        Ok(m::RunSourceOutcome::Ran { source_ran: ran })
    }
}

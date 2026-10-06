// generated from cortex v1
// model digest 80093450f0d7a03dc57722348c9507b053cfa770ba8cc2c76d862d998e927a17
// contract digest 8d562b038cb06e2577e6a148e341c1535923c082eabdc3385792c367302f8fcc
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `cortex` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: `at_least_once` is the only
//! delivery guarantee the model declares, so published events land on an append-only log and a
//! pump delivers each to every binding that reacts to it. The log is the system's observable
//! record, and so is the record of what each binding invoked. What no specification determines
//! — how an escalation event is filled, behaviour behind the ports — stays an obligation; see
//! the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// `cortex.instance.InstanceAdopted`.
    InstanceAdopted(crate::instance::InstanceAdopted),
    /// `cortex.instance.InstanceCreated`.
    InstanceCreated(crate::instance::InstanceCreated),
    /// `cortex.instance.InstanceRemoved`.
    InstanceRemoved(crate::instance::InstanceRemoved),
    /// `cortex.instance.InstanceUpdated`.
    InstanceUpdated(crate::instance::InstanceUpdated),
    /// `cortex.instance.QualityMeasured`.
    QualityMeasured(crate::instance::QualityMeasured),
    /// `cortex.instance.RunFailed`.
    RunFailed(crate::instance::RunFailed),
    /// `cortex.instance.SchemaChangesProposed`.
    SchemaChangesProposed(crate::instance::SchemaChangesProposed),
    /// `cortex.instance.SnapshotRestored`.
    SnapshotRestored(crate::instance::SnapshotRestored),
    /// `cortex.instance.SourceAdded`.
    SourceAdded(crate::instance::SourceAdded),
    /// `cortex.instance.SourceDisabled`.
    SourceDisabled(crate::instance::SourceDisabled),
    /// `cortex.instance.SourceEnabled`.
    SourceEnabled(crate::instance::SourceEnabled),
    /// `cortex.instance.SourceRan`.
    SourceRan(crate::instance::SourceRan),
}

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::InstanceAdopted(_) => "cortex.instance.InstanceAdopted",
            Self::InstanceCreated(_) => "cortex.instance.InstanceCreated",
            Self::InstanceRemoved(_) => "cortex.instance.InstanceRemoved",
            Self::InstanceUpdated(_) => "cortex.instance.InstanceUpdated",
            Self::QualityMeasured(_) => "cortex.instance.QualityMeasured",
            Self::RunFailed(_) => "cortex.instance.RunFailed",
            Self::SchemaChangesProposed(_) => "cortex.instance.SchemaChangesProposed",
            Self::SnapshotRestored(_) => "cortex.instance.SnapshotRestored",
            Self::SourceAdded(_) => "cortex.instance.SourceAdded",
            Self::SourceDisabled(_) => "cortex.instance.SourceDisabled",
            Self::SourceEnabled(_) => "cortex.instance.SourceEnabled",
            Self::SourceRan(_) => "cortex.instance.SourceRan",
        }
    }
}

impl From<crate::ports::cortex::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::cortex::PublishedEvent) -> Self {
        match event {
            crate::ports::cortex::PublishedEvent::InstanceAdopted(event) => Self::InstanceAdopted(event),
            crate::ports::cortex::PublishedEvent::InstanceCreated(event) => Self::InstanceCreated(event),
            crate::ports::cortex::PublishedEvent::InstanceRemoved(event) => Self::InstanceRemoved(event),
            crate::ports::cortex::PublishedEvent::InstanceUpdated(event) => Self::InstanceUpdated(event),
            crate::ports::cortex::PublishedEvent::QualityMeasured(event) => Self::QualityMeasured(event),
            crate::ports::cortex::PublishedEvent::RunFailed(event) => Self::RunFailed(event),
            crate::ports::cortex::PublishedEvent::SchemaChangesProposed(event) => Self::SchemaChangesProposed(event),
            crate::ports::cortex::PublishedEvent::SnapshotRestored(event) => Self::SnapshotRestored(event),
            crate::ports::cortex::PublishedEvent::SourceAdded(event) => Self::SourceAdded(event),
            crate::ports::cortex::PublishedEvent::SourceDisabled(event) => Self::SourceDisabled(event),
            crate::ports::cortex::PublishedEvent::SourceEnabled(event) => Self::SourceEnabled(event),
            crate::ports::cortex::PublishedEvent::SourceRan(event) => Self::SourceRan(event),
        }
    }
}

/// The `cortex` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<CortexBehaviors> {
    /// The `cortex` component.
    pub cortex: crate::ports::cortex::Cortex<CortexBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<CortexBehaviors> System<CortexBehaviors> {
    /// Assembles the system from its components.
    pub fn new(cortex: crate::ports::cortex::Cortex<CortexBehaviors>) -> Self {
        Self {
            cortex,
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }

    /// Takes every event the pump has already delivered off the log, in publication order.
    ///
    /// A long-running shell calls this after each `pump`, or the log holds every event the
    /// process ever published. A `pump` returns with every logged event delivered: each
    /// reacting binding has had its attempt, and a binding whose attempt stopped holds the event in
    /// its own held-back list, not on the log. Events published since the last `pump` stay on the
    /// log, so the next `pump` still delivers them; taking never skips a binding.
    pub fn take_published(&mut self) -> Vec<SystemEvent> {
        let delivered: Vec<SystemEvent> = self.published.drain(..self.cursor).collect();
        self.cursor = 0;
        delivered
    }
}

impl<CortexBehaviors> System<CortexBehaviors>
where
    CortexBehaviors: crate::instance::obligations::AddSourceBehavior + crate::instance::obligations::AdoptInstanceBehavior + crate::instance::obligations::CreateInstanceBehavior + crate::instance::obligations::EnableSourceBehavior + crate::instance::obligations::MeasureQualityBehavior + crate::instance::obligations::ProposeSchemaChangesBehavior + crate::instance::obligations::RecordFailureBehavior + crate::instance::obligations::RemoveInstanceBehavior + crate::instance::obligations::RestoreSnapshotBehavior + crate::instance::obligations::RunSourceBehavior + crate::instance::obligations::UpdateInstanceBehavior + crate::instance::obligations::InstancesQuery + crate::instance::obligations::SourcesQuery,
{
    /// Delivers until quiescent: collects every component's outbox onto the log. No binding
    /// reacts to anything this specification publishes, so collecting is the whole delivery.
    pub fn pump(&mut self) -> Result<(), crate::obligation::UnmetObligation> {
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.cortex.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}

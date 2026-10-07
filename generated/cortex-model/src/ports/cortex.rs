// generated from cortex v1
// model digest 724e4de48519154c6fb2cfa76120dddf9146deb7a9a2a170344f384298bf6ce0
// contract digest 8e1213ec135a97d97b798315901e861ab9560bb7401301d5943fb3f16838dddd
// do not edit: regenerate with `ess synthesize --layout crate`

//! cortex — the `cortex` component of `cortex` v1.
//!
//! The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
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

/// cortex — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Cortex<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Cortex<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> Cortex<B>
where
    B: crate::instance::obligations::AddSourceBehavior + crate::instance::obligations::AdoptInstanceBehavior + crate::instance::obligations::CreateInstanceBehavior + crate::instance::obligations::EnableSourceBehavior + crate::instance::obligations::MeasureQualityBehavior + crate::instance::obligations::ProposeSchemaChangesBehavior + crate::instance::obligations::RecordFailureBehavior + crate::instance::obligations::RemoveInstanceBehavior + crate::instance::obligations::RestoreSnapshotBehavior + crate::instance::obligations::RunSourceBehavior + crate::instance::obligations::UpdateInstanceBehavior + crate::instance::obligations::InstancesQuery + crate::instance::obligations::SourcesQuery,
{
    /// Accepts `cortex.instance.AddSource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_source(&mut self, input: crate::instance::AddSource) -> Result<crate::instance::AddSourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_source(input)?;
        match &outcome {
            crate::instance::AddSourceOutcome::Added { source_added, .. } => {
                self.outbox.push(PublishedEvent::SourceAdded(source_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.AdoptInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn adopt_instance(&mut self, input: crate::instance::AdoptInstance) -> Result<crate::instance::AdoptInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.adopt_instance(input)?;
        match &outcome {
            crate::instance::AdoptInstanceOutcome::NameTaken { .. } => {}
            crate::instance::AdoptInstanceOutcome::BackendMismatch { .. } => {}
            crate::instance::AdoptInstanceOutcome::StoreUnreadable { .. } => {}
            crate::instance::AdoptInstanceOutcome::StoreHeld { .. } => {}
            crate::instance::AdoptInstanceOutcome::SeedTypesMissing { .. } => {}
            crate::instance::AdoptInstanceOutcome::Adopted { instance_adopted, .. } => {
                self.outbox.push(PublishedEvent::InstanceAdopted(instance_adopted.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.CreateInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn create_instance(&mut self, input: crate::instance::CreateInstance) -> Result<crate::instance::CreateInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.create_instance(input)?;
        match &outcome {
            crate::instance::CreateInstanceOutcome::NameTaken { .. } => {}
            crate::instance::CreateInstanceOutcome::ConnectionMissing { .. } => {}
            crate::instance::CreateInstanceOutcome::SeedRefused { .. } => {}
            crate::instance::CreateInstanceOutcome::Partial { instance_created, .. } => {
                self.outbox.push(PublishedEvent::InstanceCreated(instance_created.clone()));
            }
            crate::instance::CreateInstanceOutcome::Created { instance_created, .. } => {
                self.outbox.push(PublishedEvent::InstanceCreated(instance_created.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.EnableSource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn enable_source(&mut self, input: crate::instance::EnableSource) -> Result<crate::instance::EnableSourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.enable_source(input)?;
        match &outcome {
            crate::instance::EnableSourceOutcome::Enabled { source_enabled, .. } => {
                self.outbox.push(PublishedEvent::SourceEnabled(source_enabled.clone()));
            }
            crate::instance::EnableSourceOutcome::WrongState { .. } => {}
            crate::instance::EnableSourceOutcome::NoSuchSource { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.MeasureQuality`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn measure_quality(&mut self, input: crate::instance::MeasureQuality) -> Result<crate::instance::MeasureQualityOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.measure_quality(input)?;
        match &outcome {
            crate::instance::MeasureQualityOutcome::SampleFailed { .. } => {}
            crate::instance::MeasureQualityOutcome::JudgeFailed { .. } => {}
            crate::instance::MeasureQualityOutcome::Measured { quality_measured, .. } => {
                self.outbox.push(PublishedEvent::QualityMeasured(quality_measured.clone()));
            }
            crate::instance::MeasureQualityOutcome::NoSuchInstance { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.ProposeSchemaChanges`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn propose_schema_changes(&mut self, input: crate::instance::ProposeSchemaChanges) -> Result<crate::instance::ProposeSchemaChangesOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.propose_schema_changes(input)?;
        match &outcome {
            crate::instance::ProposeSchemaChangesOutcome::SampleFailed { .. } => {}
            crate::instance::ProposeSchemaChangesOutcome::ProposeFailed { .. } => {}
            crate::instance::ProposeSchemaChangesOutcome::Proposed { schema_changes_proposed, .. } => {
                self.outbox.push(PublishedEvent::SchemaChangesProposed(schema_changes_proposed.clone()));
            }
            crate::instance::ProposeSchemaChangesOutcome::Applied { schema_changes_proposed, .. } => {
                self.outbox.push(PublishedEvent::SchemaChangesProposed(schema_changes_proposed.clone()));
            }
            crate::instance::ProposeSchemaChangesOutcome::NoSuchInstance { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.RecordFailure`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_failure(&mut self, input: crate::instance::RecordFailure) -> Result<crate::instance::RecordFailureOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_failure(input)?;
        match &outcome {
            crate::instance::RecordFailureOutcome::Disabled { source_disabled, .. } => {
                self.outbox.push(PublishedEvent::SourceDisabled(source_disabled.clone()));
            }
            crate::instance::RecordFailureOutcome::Counted { run_failed, .. } => {
                self.outbox.push(PublishedEvent::RunFailed(run_failed.clone()));
            }
            crate::instance::RecordFailureOutcome::AlreadyDisabled { .. } => {}
            crate::instance::RecordFailureOutcome::NoSuchSource { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.RemoveInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn remove_instance(&mut self, input: crate::instance::RemoveInstance) -> Result<crate::instance::RemoveInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.remove_instance(input)?;
        match &outcome {
            crate::instance::RemoveInstanceOutcome::Removed { instance_removed, .. } => {
                self.outbox.push(PublishedEvent::InstanceRemoved(instance_removed.clone()));
            }
            crate::instance::RemoveInstanceOutcome::WrongState { .. } => {}
            crate::instance::RemoveInstanceOutcome::NoSuchInstance { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.RestoreSnapshot`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn restore_snapshot(&mut self, input: crate::instance::RestoreSnapshot) -> Result<crate::instance::RestoreSnapshotOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.restore_snapshot(input)?;
        match &outcome {
            crate::instance::RestoreSnapshotOutcome::BackendUnsupported { .. } => {}
            crate::instance::RestoreSnapshotOutcome::Busy { .. } => {}
            crate::instance::RestoreSnapshotOutcome::NoSuchSnapshot { .. } => {}
            crate::instance::RestoreSnapshotOutcome::Restored { snapshot_restored, .. } => {
                self.outbox.push(PublishedEvent::SnapshotRestored(snapshot_restored.clone()));
            }
            crate::instance::RestoreSnapshotOutcome::NoSuchInstance { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.RunSource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn run_source(&mut self, input: crate::instance::RunSource) -> Result<crate::instance::RunSourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.run_source(input)?;
        match &outcome {
            crate::instance::RunSourceOutcome::FetchFailed { .. } => {}
            crate::instance::RunSourceOutcome::ExtractionFailed { .. } => {}
            crate::instance::RunSourceOutcome::ApplyRefused { .. } => {}
            crate::instance::RunSourceOutcome::Ran { source_ran, .. } => {
                self.outbox.push(PublishedEvent::SourceRan(source_ran.clone()));
            }
            crate::instance::RunSourceOutcome::Disabled { .. } => {}
            crate::instance::RunSourceOutcome::NoSuchSource { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.instance.UpdateInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn update_instance(&mut self, input: crate::instance::UpdateInstance) -> Result<crate::instance::UpdateInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.update_instance(input)?;
        match &outcome {
            crate::instance::UpdateInstanceOutcome::SeedChangeRefused { .. } => {}
            crate::instance::UpdateInstanceOutcome::NotActive { .. } => {}
            crate::instance::UpdateInstanceOutcome::Updated { instance_updated, .. } => {
                self.outbox.push(PublishedEvent::InstanceUpdated(instance_updated.clone()));
            }
            crate::instance::UpdateInstanceOutcome::NoSuchInstance { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `cortex.instance.Instances` at `read_your_writes` consistency, from the owed projection.
    pub fn instances(&self) -> Result<Vec<crate::instance::Instances>, crate::obligation::UnmetObligation> {
        self.behaviors.instances()
    }

    /// Serves `cortex.instance.Sources` at `read_your_writes` consistency, from the owed projection.
    pub fn sources(&self) -> Result<Vec<crate::instance::Sources>, crate::obligation::UnmetObligation> {
        self.behaviors.sources()
    }
}

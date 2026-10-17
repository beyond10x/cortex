// generated from cortex v1
// model digest 5b8d449a9b5dd33fe4da24d7b0507f49b8bbbd969b3c9007942d6f34fea63878
// contract digest 2f5be3eec0297f1a905f33f1e2355526c36078813e7f121ca0bb406b9b6ef9be
// do not edit: regenerate with `ess synthesize --layout crate`

//! cortex-ask — the `cortex-ask` component of `cortex` v1.
//!
//! The answer service, `cortex ask serve --spec <instance spec>`: a long-running process that answers `POST /ask` as a stream of events over the instance's store, read through the EKR MCP reader. The batch component `cortex` serves no port; this one serves the spec's `ask.listen`.
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
    /// `cortex.ask.Answered`.
    Answered(crate::ask::Answered),
}

/// cortex-ask — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct CortexAsk<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> CortexAsk<B> {
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

impl<B> CortexAsk<B>
where
    B: crate::ask::obligations::AskBehavior,
{
    /// Accepts `cortex.ask.Ask`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn ask(&mut self, input: crate::ask::Ask) -> Result<crate::ask::AskOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.ask(input)?;
        match &outcome {
            crate::ask::AskOutcome::Unidentified { .. } => {}
            crate::ask::AskOutcome::RateLimited { .. } => {}
            crate::ask::AskOutcome::LimitReached { .. } => {}
            crate::ask::AskOutcome::ModelFailed { .. } => {}
            crate::ask::AskOutcome::ReaderFailed { .. } => {}
            crate::ask::AskOutcome::Answered { answered, .. } => {
                self.outbox.push(PublishedEvent::Answered(answered.clone()));
            }
        }
        Ok(outcome)
    }
}

// generated from cortex v1
// model digest 8722ddd6d5934a5124c04d561aaa97fa387b5df3f0816c27167594157fc396fc
// contract digest 149600d03fd42e1f199297e8fbc8e7c0c9027f25b91767ca9cc19806d52e4c76
// do not edit: regenerate with `ess synthesize --layout crate`

//! cortex-ask — the `cortex-ask` component of `cortex` v1.
//!
//! The answer service, `cortex ask serve --spec <instance spec>`: a long-running process that answers `POST /ask` as a stream of events over the instance's store, read through the EKR MCP reader. The batch component `cortex` serves no port; this one serves the spec's `ask.listen`. It keeps every answer with its votes and a review queue in `ask.sqlite` in the instance directory, or, for an instance on a `postgres` store, in a PostgreSQL schema of their own through a records process launched by Connectors (`cortex ask records`), and purges them after the spec's `retention_days`.
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
    /// `cortex.ask.AnswerRecorded`.
    AnswerRecorded(crate::ask::AnswerRecorded),
    /// `cortex.ask.Answered`.
    Answered(crate::ask::Answered),
    /// `cortex.ask.Purged`.
    Purged(crate::ask::Purged),
    /// `cortex.ask.ReviewClosed`.
    ReviewClosed(crate::ask::ReviewClosed),
    /// `cortex.ask.ReviewOpened`.
    ReviewOpened(crate::ask::ReviewOpened),
    /// `cortex.ask.Voted`.
    Voted(crate::ask::Voted),
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
    B: crate::ask::obligations::AskBehavior + crate::ask::obligations::CastVoteBehavior + crate::ask::obligations::CloseReviewBehavior + crate::ask::obligations::OpenReviewBehavior + crate::ask::obligations::PurgeExpiredBehavior + crate::ask::obligations::RecordAnswerBehavior + crate::ask::obligations::OpenReviewsQuery,
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

    /// Accepts `cortex.ask.CastVote`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn cast_vote(&mut self, input: crate::ask::CastVote) -> Result<crate::ask::CastVoteOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.cast_vote(input)?;
        match &outcome {
            crate::ask::CastVoteOutcome::Unidentified { .. } => {}
            crate::ask::CastVoteOutcome::NoSuchAnswer { .. } => {}
            crate::ask::CastVoteOutcome::Changed { voted, .. } => {
                self.outbox.push(PublishedEvent::Voted(voted.clone()));
            }
            crate::ask::CastVoteOutcome::Cast { voted, .. } => {
                self.outbox.push(PublishedEvent::Voted(voted.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `cortex.ask.CloseReview`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn close_review(&mut self, input: crate::ask::CloseReview) -> Result<crate::ask::CloseReviewOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.close_review(input)?;
        match &outcome {
            crate::ask::CloseReviewOutcome::Closed { review_closed, .. } => {
                self.outbox.push(PublishedEvent::ReviewClosed(review_closed.clone()));
            }
            crate::ask::CloseReviewOutcome::WrongState { .. } => {}
            crate::ask::CloseReviewOutcome::NoSuchReview { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.ask.OpenReview`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_review(&mut self, input: crate::ask::OpenReview) -> Result<crate::ask::OpenReviewOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_review(input)?;
        match &outcome {
            crate::ask::OpenReviewOutcome::Opened { review_opened, .. } => {
                self.outbox.push(PublishedEvent::ReviewOpened(review_opened.clone()));
            }
            crate::ask::OpenReviewOutcome::Exists { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `cortex.ask.PurgeExpired`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn purge_expired(&mut self, input: crate::ask::PurgeExpired) -> Result<crate::ask::PurgeExpiredOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.purge_expired(input)?;
        match &outcome {
            crate::ask::PurgeExpiredOutcome::Purged { purged, .. } => {
                self.outbox.push(PublishedEvent::Purged(purged.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `cortex.ask.RecordAnswer`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_answer(&mut self, input: crate::ask::RecordAnswer) -> Result<crate::ask::RecordAnswerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_answer(input)?;
        match &outcome {
            crate::ask::RecordAnswerOutcome::Recorded { answer_recorded, .. } => {
                self.outbox.push(PublishedEvent::AnswerRecorded(answer_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `cortex.ask.OpenReviews` at `read_your_writes` consistency, from the owed projection.
    pub fn open_reviews(&self) -> Result<Vec<crate::ask::OpenReviews>, crate::obligation::UnmetObligation> {
        self.behaviors.open_reviews()
    }
}

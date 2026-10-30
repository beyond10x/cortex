// generated from cortex v1
// model digest 8ba1171f5da02495037a1e9a0c2c22b595c620ed913157855e46a8589db2b3b3
// contract digest 49a32b9a002eac9bb0fd15738a4936ed65cf3a2ad5dea76a21f77a182cc12559
// do not edit: regenerate with `ess synthesize --layout crate`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! preserves the trait; generated network entries supply an ephemeral store. `Context` carries the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `cortex.ask.AnswerRecord` is stored — a port the implementor provides.
///
/// Keyed by the identity `answer_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait AnswerRecordStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::ask::AnswerId) -> Option<crate::ask::AnswerRecordSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::ask::AnswerRecordSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::ask::AnswerId);
}

/// Where `cortex.ask.ReviewItem` is stored — a port the implementor provides.
///
/// Keyed by the identity `review_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ReviewItemStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::ask::ReviewId) -> Option<crate::ask::ReviewItemSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::ask::ReviewItemSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::ask::ReviewId);
}

/// Where `cortex.ask.Vote` is stored — a port the implementor provides.
///
/// Keyed by the identity `vote_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait VoteStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::ask::VoteId) -> Option<crate::ask::VoteSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::ask::VoteSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::ask::VoteId);
}

/// Where `cortex.instance.Instance` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait InstanceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::instance::InstanceName) -> Option<crate::instance::InstanceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::instance::InstanceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::instance::InstanceName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::instance::InstanceSnapshot>;
}

/// Where `cortex.instance.Source` is stored — a port the implementor provides.
///
/// Keyed by the identity `source_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SourceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::instance::SourceId) -> Option<crate::instance::SourceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::instance::SourceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::instance::SourceId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::instance::SourceSnapshot>;
}

/// The exact executing command input supplied to an external decision.
///
/// This supplies facts, not authority: the context must verify its request-bound proof.
#[derive(Debug, Clone, Copy)]
pub enum ExternalCommand<'a> {
    /// The executing `cortex.ask.Ask` input.
    CortexAskAsk(&'a crate::ask::Ask),
    /// The executing `cortex.ask.CastVote` input.
    CortexAskCastVote(&'a crate::ask::CastVote),
    /// The executing `cortex.instance.AddEvalCase` input.
    CortexInstanceAddEvalCase(&'a crate::instance::AddEvalCase),
    /// The executing `cortex.instance.AdoptInstance` input.
    CortexInstanceAdoptInstance(&'a crate::instance::AdoptInstance),
    /// The executing `cortex.instance.ApplySchemaProposals` input.
    CortexInstanceApplySchemaProposals(&'a crate::instance::ApplySchemaProposals),
    /// The executing `cortex.instance.ApproveSchemaProposals` input.
    CortexInstanceApproveSchemaProposals(&'a crate::instance::ApproveSchemaProposals),
    /// The executing `cortex.instance.CreateInstance` input.
    CortexInstanceCreateInstance(&'a crate::instance::CreateInstance),
    /// The executing `cortex.instance.MeasureQuality` input.
    CortexInstanceMeasureQuality(&'a crate::instance::MeasureQuality),
    /// The executing `cortex.instance.ProposeSchemaChanges` input.
    CortexInstanceProposeSchemaChanges(&'a crate::instance::ProposeSchemaChanges),
    /// The executing `cortex.instance.RejectSchemaProposals` input.
    CortexInstanceRejectSchemaProposals(&'a crate::instance::RejectSchemaProposals),
    /// The executing `cortex.instance.RestoreSnapshot` input.
    CortexInstanceRestoreSnapshot(&'a crate::instance::RestoreSnapshot),
    /// The executing `cortex.instance.RunEval` input.
    CortexInstanceRunEval(&'a crate::instance::RunEval),
}

impl ExternalCommand<'_> {
    /// The canonical qualified identity of this command.
    pub fn name(&self) -> &'static str {
        match self {
            Self::CortexAskAsk(_) => "cortex.ask.Ask",
            Self::CortexAskCastVote(_) => "cortex.ask.CastVote",
            Self::CortexInstanceAddEvalCase(_) => "cortex.instance.AddEvalCase",
            Self::CortexInstanceAdoptInstance(_) => "cortex.instance.AdoptInstance",
            Self::CortexInstanceApplySchemaProposals(_) => "cortex.instance.ApplySchemaProposals",
            Self::CortexInstanceApproveSchemaProposals(_) => "cortex.instance.ApproveSchemaProposals",
            Self::CortexInstanceCreateInstance(_) => "cortex.instance.CreateInstance",
            Self::CortexInstanceMeasureQuality(_) => "cortex.instance.MeasureQuality",
            Self::CortexInstanceProposeSchemaChanges(_) => "cortex.instance.ProposeSchemaChanges",
            Self::CortexInstanceRejectSchemaProposals(_) => "cortex.instance.RejectSchemaProposals",
            Self::CortexInstanceRestoreSnapshot(_) => "cortex.instance.RestoreSnapshot",
            Self::CortexInstanceRunEval(_) => "cortex.instance.RunEval",
        }
    }
}

/// What the specification leaves to the implementor's context — a port the implementor provides.
///
/// The caller's attributes, the values the model says the implementation assigns, and the answer
/// to each `external:` branch.
pub trait Context {
    /// A new `cortex.ask.AskDone`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_cortex_ask_ask_done(&mut self) -> crate::ask::AskDone;

    /// A new `cortex.instance.Instance.State`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_cortex_instance_instance_state(&mut self) -> crate::instance::InstanceState;

    /// A new `cortex.instance.Source.State`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_cortex_instance_source_state(&mut self) -> crate::instance::SourceState;

    /// A new `Decimal`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_decimal(&mut self) -> crate::primitives::Decimal;

    /// A new `Integer`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_integer(&mut self) -> i64;

    /// A `Optional<Decimal>` the model says the implementation assigns — a `{generated: true}` value —
    /// or `None` where there is nothing to report.
    fn generate_optional_decimal(&mut self) -> Option<crate::primitives::Decimal>;

    /// A `Optional<Integer>` the model says the implementation assigns — a `{generated: true}` value —
    /// or `None` where there is nothing to report.
    fn generate_optional_integer(&mut self) -> Option<i64>;

    /// A `Optional<Timestamp>` the model says the implementation assigns — a `{generated: true}` value —
    /// or `None` where there is nothing to report.
    fn generate_optional_timestamp(&mut self) -> Option<crate::primitives::Timestamp>;

    /// A new `String`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_string(&mut self) -> String;

    /// A new `Timestamp`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_timestamp(&mut self) -> crate::primitives::Timestamp;

    /// Whether the external branch `outcome` of `command` is taken on this invocation.
    ///
    /// Asked in declaration order, before the branch's input guard is read; the first branch
    /// answered `true` whose guard holds is taken. A test forces a branch by answering `true`
    /// for it alone; a deployment asks whatever decides it.
    fn external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> bool;
}

/// Context answers that may be unavailable, without fabricated values.
/// Existing `Context` implementations receive the blanket adapter.
pub trait TryContext {
/// Assigns the value, or names the unavailable answer.
fn try_generate_cortex_ask_ask_done(&mut self) -> Result<crate::ask::AskDone, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_cortex_instance_instance_state(&mut self) -> Result<crate::instance::InstanceState, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_cortex_instance_source_state(&mut self) -> Result<crate::instance::SourceState, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_decimal(&mut self) -> Result<crate::primitives::Decimal, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_integer(&mut self) -> Result<i64, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_optional_decimal(&mut self) -> Result<Option<crate::primitives::Decimal>, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_optional_integer(&mut self) -> Result<Option<i64>, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_optional_timestamp(&mut self) -> Result<Option<crate::primitives::Timestamp>, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_string(&mut self) -> Result<String, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_timestamp(&mut self) -> Result<crate::primitives::Timestamp, UnmetObligation>;
/// Decides the named external branch, or names the unavailable answer.
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation>;
}

impl<T: Context + ?Sized> TryContext for T {
fn try_generate_cortex_ask_ask_done(&mut self) -> Result<crate::ask::AskDone, UnmetObligation> { Ok(Context::generate_cortex_ask_ask_done(self)) }
fn try_generate_cortex_instance_instance_state(&mut self) -> Result<crate::instance::InstanceState, UnmetObligation> { Ok(Context::generate_cortex_instance_instance_state(self)) }
fn try_generate_cortex_instance_source_state(&mut self) -> Result<crate::instance::SourceState, UnmetObligation> { Ok(Context::generate_cortex_instance_source_state(self)) }
fn try_generate_decimal(&mut self) -> Result<crate::primitives::Decimal, UnmetObligation> { Ok(Context::generate_decimal(self)) }
fn try_generate_integer(&mut self) -> Result<i64, UnmetObligation> { Ok(Context::generate_integer(self)) }
fn try_generate_optional_decimal(&mut self) -> Result<Option<crate::primitives::Decimal>, UnmetObligation> { Ok(Context::generate_optional_decimal(self)) }
fn try_generate_optional_integer(&mut self) -> Result<Option<i64>, UnmetObligation> { Ok(Context::generate_optional_integer(self)) }
fn try_generate_optional_timestamp(&mut self) -> Result<Option<crate::primitives::Timestamp>, UnmetObligation> { Ok(Context::generate_optional_timestamp(self)) }
fn try_generate_string(&mut self) -> Result<String, UnmetObligation> { Ok(Context::generate_string(self)) }
fn try_generate_timestamp(&mut self) -> Result<crate::primitives::Timestamp, UnmetObligation> { Ok(Context::generate_timestamp(self)) }
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation> { Ok(Context::external(self, command, outcome)) }
}

/// An unavailable runtime context answer, rather than a new planned capability.
pub fn unmet_context(source: &'static str) -> UnmetObligation { UnmetObligation { capability: "context answer", source } }

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes,
/// `TryContext` (or its legacy `Context` blanket adapter) where one asks it anything, and every `…Behavior` and `…Query` trait the plan still
/// owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage and context ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

/// `cortex.ask.Ask`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::AskBehavior for Generated<P>
where
    P: TryContext,
{
    fn ask(&mut self, input: crate::ask::Ask) -> Result<crate::ask::AskOutcome, UnmetObligation> {
        let _ = &input;
        // `unidentified`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "unidentified")? {
            return Ok(crate::ask::AskOutcome::Unidentified { error: crate::ask::Unidentified { header: self.ports.try_generate_string()? } });
        }
        // `rate-limited`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "rate-limited")? {
            return Ok(crate::ask::AskOutcome::RateLimited { error: crate::ask::RateLimited { identity: input.identity.clone(), retry_after: self.ports.try_generate_integer()? } });
        }
        // `busy`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "busy")? {
            return Ok(crate::ask::AskOutcome::Busy { error: crate::ask::Busy { max_in_flight: self.ports.try_generate_integer()? } });
        }
        // `limit-reached`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "limit-reached")? {
            return Ok(crate::ask::AskOutcome::LimitReached { error: crate::ask::LimitReached { limit: self.ports.try_generate_string()? } });
        }
        // `model-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "model-failed")? {
            return Ok(crate::ask::AskOutcome::ModelFailed { error: crate::ask::ModelFailed { reason: self.ports.try_generate_string()? } });
        }
        // `reader-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "reader-failed")? {
            return Ok(crate::ask::AskOutcome::ReaderFailed { error: crate::ask::ReaderFailed { reason: self.ports.try_generate_string()? } });
        }
        // `cut-off`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "cut-off")? {
            return Ok(crate::ask::AskOutcome::CutOff { error: crate::ask::CutOff { finish_reason: self.ports.try_generate_string()?, steps: self.ports.try_generate_integer()?, input_tokens: self.ports.try_generate_integer()?, output_tokens: self.ports.try_generate_integer()?, cached_tokens: self.ports.try_generate_integer()?, context_tokens: self.ports.try_generate_integer()? } });
        }
        // `client-closed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskAsk(&input), "client-closed")? {
            return Ok(crate::ask::AskOutcome::ClientClosed { error: crate::ask::ClientClosed { steps: self.ports.try_generate_integer()?, input_tokens: self.ports.try_generate_integer()?, output_tokens: self.ports.try_generate_integer()?, cached_tokens: self.ports.try_generate_integer()?, context_tokens: self.ports.try_generate_integer()? } });
        }
        // `answered`: the default.
        return Ok(crate::ask::AskOutcome::Answered { answered: crate::ask::Answered { identity: input.identity.clone(), done: self.ports.try_generate_cortex_ask_ask_done()? } });
    }
}

/// `cortex.ask.CastVote`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::CastVoteBehavior for Generated<P>
where
    P: TryContext + VoteStorage,
{
    fn cast_vote(&mut self, input: crate::ask::CastVote) -> Result<crate::ask::CastVoteOutcome, UnmetObligation> {
        let _ = &input;
        // `unidentified`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskCastVote(&input), "unidentified")? {
            return Ok(crate::ask::CastVoteOutcome::Unidentified { error: crate::ask::Unidentified { header: self.ports.try_generate_string()? } });
        }
        // `no-such-answer`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexAskCastVote(&input), "no-such-answer")? {
            return Ok(crate::ask::CastVoteOutcome::NoSuchAnswer { error: crate::ask::AnswerNotFound { answer_id: input.answer_id.clone() } });
        }
        // `changed`: the default.
        let Some(held) = VoteStorage::get(&self.ports, &input.vote_id) else {
            let identity: crate::ask::VoteId = input.vote_id.clone();
            let data = crate::ask::VoteData {
                vote_id: identity.clone(),
                answer_id: input.answer_id.clone(),
                identity: input.identity.clone(),
                value: input.value.clone(),
                comment: input.comment.clone(),
                at: self.ports.try_generate_timestamp()?,
            };
            let answer = crate::ask::CastVoteOutcome::Cast { voted: crate::ask::Voted { vote_id: identity.clone(), answer_id: input.answer_id.clone(), value: input.value.clone() } };
            VoteStorage::put(&mut self.ports, crate::ask::AnyVote::Cast(crate::ask::Vote::new(data)).snapshot());
            return Ok(answer);
        };
        let _ = &held;
        let mut next = held;
        next.data.value = input.value.clone();
        next.data.comment = input.comment.clone();
        next.data.at = self.ports.try_generate_timestamp()?;
        let answer = crate::ask::CastVoteOutcome::Changed { voted: crate::ask::Voted { vote_id: input.vote_id.clone(), answer_id: input.answer_id.clone(), value: input.value.clone() } };
        VoteStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.ask.CloseReview`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::CloseReviewBehavior for Generated<P>
where
    P: TryContext + ReviewItemStorage,
{
    fn close_review(&mut self, input: crate::ask::CloseReview) -> Result<crate::ask::CloseReviewOutcome, UnmetObligation> {
        let _ = &input;
        // `closed`: the default.
        let Some(held) = ReviewItemStorage::get(&self.ports, &input.review_id) else {
            return Ok(crate::ask::CloseReviewOutcome::NoSuchReview { error: crate::ask::ReviewNotFound { review_id: input.review_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::ask::AnyReviewItem::Open(instance) => crate::ask::AnyReviewItem::Closed(instance.close()),
            _ => return Ok(crate::ask::CloseReviewOutcome::WrongState { error: crate::ask::ReviewNotOpen { review_id: input.review_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.close_reason = Some(input.reason.clone());
        next.data.closed_at = self.ports.try_generate_optional_timestamp()?;
        next.data.expect_facts = Some(input.expect_facts.clone());
        next.data.expect_nodes = Some(input.expect_nodes.clone());
        let answer = crate::ask::CloseReviewOutcome::Closed { review_closed: crate::ask::ReviewClosed { review_id: input.review_id.clone(), reason: input.reason.clone() } };
        ReviewItemStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.ask.OpenReview`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::OpenReviewBehavior for Generated<P>
where
    P: TryContext + ReviewItemStorage,
{
    fn open_review(&mut self, input: crate::ask::OpenReview) -> Result<crate::ask::OpenReviewOutcome, UnmetObligation> {
        let _ = &input;
        // `exists`: an identity a record already carries, before any branch is taken.
        if ReviewItemStorage::get(&self.ports, &input.review_id).is_some() {
            return Ok(crate::ask::OpenReviewOutcome::Exists { error: crate::ask::ReviewExists { review_id: input.review_id.clone() } });
        }
        // `opened`: the default.
        let identity: crate::ask::ReviewId = input.review_id.clone();
        let data = crate::ask::ReviewItemData {
            review_id: identity.clone(),
            answer_id: input.answer_id.clone(),
            cause: input.cause.clone(),
            opened_at: self.ports.try_generate_timestamp()?,
            close_reason: None,
            closed_at: None,
            expect_facts: None,
            expect_nodes: None,
        };
        let answer = crate::ask::OpenReviewOutcome::Opened { review_opened: crate::ask::ReviewOpened { review_id: identity.clone(), answer_id: input.answer_id.clone(), cause: input.cause.clone() } };
        ReviewItemStorage::put(&mut self.ports, crate::ask::AnyReviewItem::Open(crate::ask::ReviewItem::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `cortex.ask.PurgeExpired`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::PurgeExpiredBehavior for Generated<P>
where
    P: TryContext,
{
    fn purge_expired(&mut self, input: crate::ask::PurgeExpired) -> Result<crate::ask::PurgeExpiredOutcome, UnmetObligation> {
        let _ = &input;
        // `purged`: the default.
        return Ok(crate::ask::PurgeExpiredOutcome::Purged { purged: crate::ask::Purged { before: input.before.clone(), records: self.ports.try_generate_integer()?, votes: self.ports.try_generate_integer()?, reviews: self.ports.try_generate_integer()? } });
    }
}

/// `cortex.ask.RecordAnswer`, generated: every outcome is one the specification fully determines.
impl<P> crate::ask::obligations::RecordAnswerBehavior for Generated<P>
where
    P: AnswerRecordStorage,
{
    fn record_answer(&mut self, input: crate::ask::RecordAnswer) -> Result<crate::ask::RecordAnswerOutcome, UnmetObligation> {
        let _ = &input;
        // `recorded`: the default.
        let identity: crate::ask::AnswerId = input.answer_id.clone();
        let data = crate::ask::AnswerRecordData {
            answer_id: identity.clone(),
            instance_name: input.instance_name.clone(),
            identity: input.identity.clone(),
            question: input.question.clone(),
            answer: input.answer.clone(),
            reason: input.reason.clone(),
            outcome: input.outcome.clone(),
            tool_calls: input.tool_calls.clone(),
            latency_ms: input.latency_ms.clone(),
            input_tokens: input.input_tokens.clone(),
            output_tokens: input.output_tokens.clone(),
            cached_tokens: input.cached_tokens.clone(),
            context_tokens: input.context_tokens.clone(),
            steps: input.steps.clone(),
            at: input.at.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::ask::RecordAnswerOutcome::Recorded { answer_recorded: crate::ask::AnswerRecorded { answer_id: identity.clone(), outcome: input.outcome.clone() } };
        AnswerRecordStorage::put(&mut self.ports, crate::ask::AnyAnswerRecord::Kept(crate::ask::AnswerRecord::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `cortex.instance.AddEvalCase`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::AddEvalCaseBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn add_eval_case(&mut self, input: crate::instance::AddEvalCase) -> Result<crate::instance::AddEvalCaseOutcome, UnmetObligation> {
        let _ = &input;
        // `set-invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAddEvalCase(&input), "set-invalid")? {
            return Ok(crate::instance::AddEvalCaseOutcome::SetInvalid { error: crate::instance::EvalSetInvalid { reason: self.ports.try_generate_string()? } });
        }
        // `no-such-review`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAddEvalCase(&input), "no-such-review")? {
            return Ok(crate::instance::AddEvalCaseOutcome::NoSuchReview { error: crate::instance::EvalReviewNotFound { review_id: input.from_review.clone() } });
        }
        // `review-open`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAddEvalCase(&input), "review-open")? {
            return Ok(crate::instance::AddEvalCaseOutcome::ReviewOpen { error: crate::instance::EvalReviewOpen { review_id: input.from_review.clone() } });
        }
        // `no-expectations`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAddEvalCase(&input), "no-expectations")? {
            return Ok(crate::instance::AddEvalCaseOutcome::NoExpectations { error: crate::instance::EvalNoExpectations { review_id: input.from_review.clone() } });
        }
        // `case-exists`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAddEvalCase(&input), "case-exists")? {
            return Ok(crate::instance::AddEvalCaseOutcome::CaseExists { error: crate::instance::EvalCaseExists { review_id: input.from_review.clone() } });
        }
        // `added`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::AddEvalCaseOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::AddEvalCaseOutcome::Added { eval_case_added: crate::instance::EvalCaseAdded { name: input.name.clone(), set: input.set.clone(), review_id: input.from_review.clone(), id: self.ports.try_generate_string()?, cases: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.AddSource`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::AddSourceBehavior for Generated<P>
where
    P: SourceStorage,
{
    fn add_source(&mut self, input: crate::instance::AddSource) -> Result<crate::instance::AddSourceOutcome, UnmetObligation> {
        let _ = &input;
        // `added`: the default.
        let identity: crate::instance::SourceId = input.source_id.clone();
        let data = crate::instance::SourceData {
            source_id: identity.clone(),
            instance_name: input.instance_name.clone(),
            name: input.name.clone(),
            kind: input.kind.clone(),
            schedule: input.schedule.clone(),
            runs: 0,
            consecutive_failures: 0,
            last_outcome: None,
            last_run_at: None,
            last_reason: None,
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::instance::AddSourceOutcome::Added { source_added: crate::instance::SourceAdded { source_id: identity.clone(), instance_name: input.instance_name.clone() } };
        SourceStorage::put(&mut self.ports, crate::instance::AnySource::Enabled(crate::instance::Source::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `cortex.instance.AdoptInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::AdoptInstanceBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn adopt_instance(&mut self, input: crate::instance::AdoptInstance) -> Result<crate::instance::AdoptInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `name-taken`: an identity a record already carries, before any branch is taken.
        if InstanceStorage::get(&self.ports, &input.name).is_some() {
            return Ok(crate::instance::AdoptInstanceOutcome::NameTaken { error: crate::instance::NameTaken { name: input.name.clone() } });
        }
        // `backend-mismatch`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAdoptInstance(&input), "backend-mismatch")? {
            return Ok(crate::instance::AdoptInstanceOutcome::BackendMismatch { error: crate::instance::BackendMismatch { reason: self.ports.try_generate_string()? } });
        }
        // `store-unreadable`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAdoptInstance(&input), "store-unreadable")? {
            return Ok(crate::instance::AdoptInstanceOutcome::StoreUnreadable { error: crate::instance::StoreUnreadable { reason: self.ports.try_generate_string()? } });
        }
        // `store-held`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAdoptInstance(&input), "store-held")? {
            return Ok(crate::instance::AdoptInstanceOutcome::StoreHeld { error: crate::instance::StoreHeld { name: self.ports.try_generate_string()? } });
        }
        // `seed-types-missing`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceAdoptInstance(&input), "seed-types-missing")? {
            return Ok(crate::instance::AdoptInstanceOutcome::SeedTypesMissing { error: crate::instance::SeedTypesMissing { types: self.ports.try_generate_string()? } });
        }
        // `adopted`: the default.
        let identity: crate::instance::InstanceName = input.name.clone();
        let data = crate::instance::InstanceData {
            name: identity.clone(),
            description: input.description.clone(),
            model: input.model.clone(),
            ekr_version: input.ekr_version.clone(),
            seed_digest: input.seed_digest.clone(),
        };
        let answer = crate::instance::AdoptInstanceOutcome::Adopted { instance_adopted: crate::instance::InstanceAdopted { name: identity.clone(), revision: self.ports.try_generate_integer()?, view_port: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, crate::instance::AnyInstance::Active(crate::instance::Instance::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `cortex.instance.ApplySchemaProposals`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::ApplySchemaProposalsBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn apply_schema_proposals(&mut self, input: crate::instance::ApplySchemaProposals) -> Result<crate::instance::ApplySchemaProposalsOutcome, UnmetObligation> {
        let _ = &input;
        // `no-such-run`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApplySchemaProposals(&input), "no-such-run")? {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::NoSuchRun { error: crate::instance::SchemaRunNotFound { stamp: input.stamp.clone(), reason: self.ports.try_generate_string()? } });
        }
        // `not-approved`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApplySchemaProposals(&input), "not-approved")? {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::NotApproved { error: crate::instance::SchemaProposalsNotApproved { reason: self.ports.try_generate_string()? } });
        }
        // `gate-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApplySchemaProposals(&input), "gate-failed")? {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::GateFailed { error: crate::instance::SchemaGateFailed { reason: self.ports.try_generate_string()? } });
        }
        // `apply-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApplySchemaProposals(&input), "apply-failed")? {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::ApplyFailed { error: crate::instance::SchemaApplyFailed { reason: self.ports.try_generate_string()? } });
        }
        // `store-outcome-unknown`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApplySchemaProposals(&input), "store-outcome-unknown")? {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::StoreOutcomeUnknown { error: crate::instance::StoreOutcomeUnknown { reason: self.ports.try_generate_string()? } });
        }
        // `applied`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::ApplySchemaProposalsOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::ApplySchemaProposalsOutcome::Applied { schema_proposals_applied: crate::instance::SchemaProposalsApplied { name: input.name.clone(), stamp: input.stamp.clone(), applied: self.ports.try_generate_integer()?, refused: self.ports.try_generate_integer()?, recorded_only: self.ports.try_generate_integer()?, invalid: self.ports.try_generate_integer()?, revision: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.ApproveSchemaProposals`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::ApproveSchemaProposalsBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn approve_schema_proposals(&mut self, input: crate::instance::ApproveSchemaProposals) -> Result<crate::instance::ApproveSchemaProposalsOutcome, UnmetObligation> {
        let _ = &input;
        // `no-such-run`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApproveSchemaProposals(&input), "no-such-run")? {
            return Ok(crate::instance::ApproveSchemaProposalsOutcome::NoSuchRun { error: crate::instance::SchemaRunNotFound { stamp: input.stamp.clone(), reason: self.ports.try_generate_string()? } });
        }
        // `not-pending`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceApproveSchemaProposals(&input), "not-pending")? {
            return Ok(crate::instance::ApproveSchemaProposalsOutcome::NotPending { error: crate::instance::SchemaProposalsNotPending { reason: self.ports.try_generate_string()? } });
        }
        // `approved`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::ApproveSchemaProposalsOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::ApproveSchemaProposalsOutcome::Approved { schema_proposals_decided: crate::instance::SchemaProposalsDecided { name: input.name.clone(), stamp: input.stamp.clone(), decision: self.ports.try_generate_string()?, decided: self.ports.try_generate_integer()?, pending: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.CreateInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::CreateInstanceBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn create_instance(&mut self, input: crate::instance::CreateInstance) -> Result<crate::instance::CreateInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `name-taken`: an identity a record already carries, before any branch is taken.
        if InstanceStorage::get(&self.ports, &input.name).is_some() {
            return Ok(crate::instance::CreateInstanceOutcome::NameTaken { error: crate::instance::NameTaken { name: input.name.clone() } });
        }
        // `connection-missing`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceCreateInstance(&input), "connection-missing")? {
            return Ok(crate::instance::CreateInstanceOutcome::ConnectionMissing { error: crate::instance::ConnectionMissing { connection: self.ports.try_generate_string()?, reason: self.ports.try_generate_string()? } });
        }
        // `seed-refused`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceCreateInstance(&input), "seed-refused")? {
            return Ok(crate::instance::CreateInstanceOutcome::SeedRefused { error: crate::instance::SeedRefused { reason: self.ports.try_generate_string()? } });
        }
        // `partial`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceCreateInstance(&input), "partial")? {
            let identity: crate::instance::InstanceName = input.name.clone();
            let data = crate::instance::InstanceData {
                name: identity.clone(),
                description: input.description.clone(),
                model: input.model.clone(),
                ekr_version: input.ekr_version.clone(),
                seed_digest: input.seed_digest.clone(),
            };
            let answer = crate::instance::CreateInstanceOutcome::Partial { instance_created: crate::instance::InstanceCreated { name: identity.clone(), view_port: self.ports.try_generate_integer()? } };
            InstanceStorage::put(&mut self.ports, crate::instance::AnyInstance::Active(crate::instance::Instance::new(data)).snapshot());
            return Ok(answer);
        }
        // `created`: the default.
        let identity: crate::instance::InstanceName = input.name.clone();
        let data = crate::instance::InstanceData {
            name: identity.clone(),
            description: input.description.clone(),
            model: input.model.clone(),
            ekr_version: input.ekr_version.clone(),
            seed_digest: input.seed_digest.clone(),
        };
        let answer = crate::instance::CreateInstanceOutcome::Created { instance_created: crate::instance::InstanceCreated { name: identity.clone(), view_port: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, crate::instance::AnyInstance::Active(crate::instance::Instance::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `cortex.instance.EnableSource`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::EnableSourceBehavior for Generated<P>
where
    P: TryContext + SourceStorage,
{
    fn enable_source(&mut self, input: crate::instance::EnableSource) -> Result<crate::instance::EnableSourceOutcome, UnmetObligation> {
        let _ = &input;
        // `enabled`: the default.
        let Some(held) = SourceStorage::get(&self.ports, &input.source_id) else {
            return Ok(crate::instance::EnableSourceOutcome::NoSuchSource { error: crate::instance::SourceNotFound { source_id: input.source_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::instance::AnySource::Disabled(instance) => crate::instance::AnySource::Enabled(instance.enable()),
            _ => return Ok(crate::instance::EnableSourceOutcome::WrongState { error: crate::instance::SourceNotDisabled { state: self.ports.try_generate_cortex_instance_source_state()? } }),
        };
        let mut next = moved.snapshot();
        next.data.consecutive_failures = 0;
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::instance::EnableSourceOutcome::Enabled { source_enabled: crate::instance::SourceEnabled { source_id: input.source_id.clone() } };
        SourceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.MeasureQuality`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::MeasureQualityBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn measure_quality(&mut self, input: crate::instance::MeasureQuality) -> Result<crate::instance::MeasureQualityOutcome, UnmetObligation> {
        let _ = &input;
        // `sample-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceMeasureQuality(&input), "sample-failed")? {
            return Ok(crate::instance::MeasureQualityOutcome::SampleFailed { error: crate::instance::SampleFailed { reason: self.ports.try_generate_string()? } });
        }
        // `judge-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceMeasureQuality(&input), "judge-failed")? {
            return Ok(crate::instance::MeasureQualityOutcome::JudgeFailed { error: crate::instance::JudgeFailed { reason: self.ports.try_generate_string()? } });
        }
        // `measured`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::MeasureQualityOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::MeasureQualityOutcome::Measured { quality_measured: crate::instance::QualityMeasured { name: input.name.clone(), stamp: self.ports.try_generate_string()?, revision: self.ports.try_generate_integer()?, seed: self.ports.try_generate_integer()?, judged: self.ports.try_generate_integer()?, passed: self.ports.try_generate_integer()?, unclear: self.ports.try_generate_integer()?, rate: self.ports.try_generate_optional_decimal()?, lower: self.ports.try_generate_decimal()?, upper: self.ports.try_generate_decimal()?, cost_usd: self.ports.try_generate_optional_decimal()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.ProposeSchemaChanges`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::ProposeSchemaChangesBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn propose_schema_changes(&mut self, input: crate::instance::ProposeSchemaChanges) -> Result<crate::instance::ProposeSchemaChangesOutcome, UnmetObligation> {
        let _ = &input;
        // `sample-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceProposeSchemaChanges(&input), "sample-failed")? {
            return Ok(crate::instance::ProposeSchemaChangesOutcome::SampleFailed { error: crate::instance::SampleFailed { reason: self.ports.try_generate_string()? } });
        }
        // `propose-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceProposeSchemaChanges(&input), "propose-failed")? {
            return Ok(crate::instance::ProposeSchemaChangesOutcome::ProposeFailed { error: crate::instance::ProposeFailed { reason: self.ports.try_generate_string()? } });
        }
        // `proposed`: an accepting branch, in declaration order.
        if decided(Some(&input.dry_run).map(|value| *value), "cortex.instance.ProposeSchemaChanges")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::ProposeSchemaChangesOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
            };
            let _ = &held;
            let next = held;
            let answer = crate::instance::ProposeSchemaChangesOutcome::Proposed { schema_changes_proposed: crate::instance::SchemaChangesProposed { name: input.name.clone(), stamp: self.ports.try_generate_string()?, revision: self.ports.try_generate_integer()?, proposed: self.ports.try_generate_integer()?, applied: self.ports.try_generate_integer()?, refused: self.ports.try_generate_integer()?, recorded_only: self.ports.try_generate_integer()?, invalid: self.ports.try_generate_integer()?, pending: self.ports.try_generate_integer()?, dropped: self.ports.try_generate_integer()?, cost_usd: self.ports.try_generate_optional_decimal()? } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `reviewed`: an accepting branch, in declaration order.
        if decided(Some(&input.review).map(|value| *value), "cortex.instance.ProposeSchemaChanges")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::ProposeSchemaChangesOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
            };
            let _ = &held;
            let next = held;
            let answer = crate::instance::ProposeSchemaChangesOutcome::Reviewed { schema_changes_proposed: crate::instance::SchemaChangesProposed { name: input.name.clone(), stamp: self.ports.try_generate_string()?, revision: self.ports.try_generate_integer()?, proposed: self.ports.try_generate_integer()?, applied: self.ports.try_generate_integer()?, refused: self.ports.try_generate_integer()?, recorded_only: self.ports.try_generate_integer()?, invalid: self.ports.try_generate_integer()?, pending: self.ports.try_generate_integer()?, dropped: self.ports.try_generate_integer()?, cost_usd: self.ports.try_generate_optional_decimal()? } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `applied`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::ProposeSchemaChangesOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::ProposeSchemaChangesOutcome::Applied { schema_changes_proposed: crate::instance::SchemaChangesProposed { name: input.name.clone(), stamp: self.ports.try_generate_string()?, revision: self.ports.try_generate_integer()?, proposed: self.ports.try_generate_integer()?, applied: self.ports.try_generate_integer()?, refused: self.ports.try_generate_integer()?, recorded_only: self.ports.try_generate_integer()?, invalid: self.ports.try_generate_integer()?, pending: self.ports.try_generate_integer()?, dropped: self.ports.try_generate_integer()?, cost_usd: self.ports.try_generate_optional_decimal()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::instance::obligations::RecordFailureBehavior> crate::instance::obligations::RecordFailureBehavior for Generated<P> {
    fn record_failure(&mut self, input: crate::instance::RecordFailure) -> Result<crate::instance::RecordFailureOutcome, UnmetObligation> {
        crate::instance::obligations::RecordFailureBehavior::record_failure(&mut self.ports, input)
    }
}

/// `cortex.instance.RejectSchemaProposals`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::RejectSchemaProposalsBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn reject_schema_proposals(&mut self, input: crate::instance::RejectSchemaProposals) -> Result<crate::instance::RejectSchemaProposalsOutcome, UnmetObligation> {
        let _ = &input;
        // `no-such-run`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRejectSchemaProposals(&input), "no-such-run")? {
            return Ok(crate::instance::RejectSchemaProposalsOutcome::NoSuchRun { error: crate::instance::SchemaRunNotFound { stamp: input.stamp.clone(), reason: self.ports.try_generate_string()? } });
        }
        // `not-pending`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRejectSchemaProposals(&input), "not-pending")? {
            return Ok(crate::instance::RejectSchemaProposalsOutcome::NotPending { error: crate::instance::SchemaProposalsNotPending { reason: self.ports.try_generate_string()? } });
        }
        // `rejected`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::RejectSchemaProposalsOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::RejectSchemaProposalsOutcome::Rejected { schema_proposals_decided: crate::instance::SchemaProposalsDecided { name: input.name.clone(), stamp: input.stamp.clone(), decision: self.ports.try_generate_string()?, decided: self.ports.try_generate_integer()?, pending: self.ports.try_generate_integer()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.RemoveInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::RemoveInstanceBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn remove_instance(&mut self, input: crate::instance::RemoveInstance) -> Result<crate::instance::RemoveInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `removed`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::RemoveInstanceOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::instance::AnyInstance::Active(instance) => crate::instance::AnyInstance::Removed(instance.remove()),
            _ => return Ok(crate::instance::RemoveInstanceOutcome::WrongState { error: crate::instance::InstanceNotActive { state: self.ports.try_generate_cortex_instance_instance_state()? } }),
        };
        let next = moved.snapshot();
        let answer = crate::instance::RemoveInstanceOutcome::Removed { instance_removed: crate::instance::InstanceRemoved { name: input.name.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.RestoreSnapshot`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::RestoreSnapshotBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn restore_snapshot(&mut self, input: crate::instance::RestoreSnapshot) -> Result<crate::instance::RestoreSnapshotOutcome, UnmetObligation> {
        let _ = &input;
        // `backend-unsupported`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRestoreSnapshot(&input), "backend-unsupported")? {
            return Ok(crate::instance::RestoreSnapshotOutcome::BackendUnsupported { error: crate::instance::RestoreUnsupported { name: input.name.clone() } });
        }
        // `busy`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRestoreSnapshot(&input), "busy")? {
            return Ok(crate::instance::RestoreSnapshotOutcome::Busy { error: crate::instance::InstanceBusy { reason: self.ports.try_generate_string()? } });
        }
        // `no-such-snapshot`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRestoreSnapshot(&input), "no-such-snapshot")? {
            return Ok(crate::instance::RestoreSnapshotOutcome::NoSuchSnapshot { error: crate::instance::SnapshotNotFound { snapshot: input.snapshot.clone() } });
        }
        // `restored`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::RestoreSnapshotOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::RestoreSnapshotOutcome::Restored { snapshot_restored: crate::instance::SnapshotRestored { name: input.name.clone(), snapshot: input.snapshot.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `cortex.instance.RunEval`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::RunEvalBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn run_eval(&mut self, input: crate::instance::RunEval) -> Result<crate::instance::RunEvalOutcome, UnmetObligation> {
        let _ = &input;
        // `set-invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRunEval(&input), "set-invalid")? {
            return Ok(crate::instance::RunEvalOutcome::SetInvalid { error: crate::instance::EvalSetInvalid { reason: self.ports.try_generate_string()? } });
        }
        // `runner-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::CortexInstanceRunEval(&input), "runner-failed")? {
            return Ok(crate::instance::RunEvalOutcome::RunnerFailed { error: crate::instance::EvalRunnerFailed { reason: self.ports.try_generate_string()? } });
        }
        // `evaluated`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::RunEvalOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        let answer = crate::instance::RunEvalOutcome::Evaluated { eval_ran: crate::instance::EvalRan { name: input.name.clone(), stamp: self.ports.try_generate_string()?, runner: input.runner.clone(), cases: self.ports.try_generate_integer()?, passed: self.ports.try_generate_integer()?, facts_expected: self.ports.try_generate_integer()?, facts_found: self.ports.try_generate_integer()?, nodes_expected: self.ports.try_generate_integer()?, nodes_found: self.ports.try_generate_integer()?, model_calls: self.ports.try_generate_integer()?, input_tokens: self.ports.try_generate_optional_integer()?, output_tokens: self.ports.try_generate_optional_integer()?, cost_usd: self.ports.try_generate_optional_decimal()? } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::instance::obligations::RunSourceBehavior> crate::instance::obligations::RunSourceBehavior for Generated<P> {
    fn run_source(&mut self, input: crate::instance::RunSource) -> Result<crate::instance::RunSourceOutcome, UnmetObligation> {
        crate::instance::obligations::RunSourceBehavior::run_source(&mut self.ports, input)
    }
}

/// `cortex.instance.UpdateInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::UpdateInstanceBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn update_instance(&mut self, input: crate::instance::UpdateInstance) -> Result<crate::instance::UpdateInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row, read before the branches that select by it.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::UpdateInstanceOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        // `seed-change-refused`: selected by the addressed row.
        if decided(equal(Some(&held.data.seed_digest).map(|value| value.clone()), Some(&input.seed_digest).map(|value| value.clone())).map(|value| !value), "cortex.instance.UpdateInstance")? {
            return Ok(crate::instance::UpdateInstanceOutcome::SeedChangeRefused { error: crate::instance::SeedChangeRefused { name: input.name.clone() } });
        }
        // `not-active`: selected by the addressed row.
        if decided(equal(Some(&held.state).map(|value| match value { crate::instance::InstanceState::Active => "Active", crate::instance::InstanceState::Removed => "Removed" }.to_owned()), Some("Removed".to_owned())), "cortex.instance.UpdateInstance")? {
            return Ok(crate::instance::UpdateInstanceOutcome::NotActive { error: crate::instance::InstanceNotActive { state: self.ports.try_generate_cortex_instance_instance_state()? } });
        }
        // `updated`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::UpdateInstanceOutcome::NoSuchInstance { error: crate::instance::InstanceNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let mut next = held;
        next.data.description = input.description.clone();
        next.data.model = input.model.clone();
        let answer = crate::instance::UpdateInstanceOutcome::Updated { instance_updated: crate::instance::InstanceUpdated { name: input.name.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::ask::obligations::OpenReviewsQuery> crate::ask::obligations::OpenReviewsQuery for Generated<P> {
    fn open_reviews(&self) -> Result<Vec<crate::ask::OpenReviews>, UnmetObligation> {
        crate::ask::obligations::OpenReviewsQuery::open_reviews(&self.ports)
    }
}

/// `cortex.instance.Instances`, generated: every row is one the specification fully determines from the stored `cortex.instance.Instance`s.
impl<P> crate::instance::obligations::InstancesQuery for Generated<P>
where
    P: InstanceStorage,
{
    fn instances(&self) -> Result<Vec<crate::instance::Instances>, UnmetObligation> {
        let admitted = InstanceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::instance::Instances {
                name: held.data.name,
                description: held.data.description,
                model: held.data.model,
                ekr_version: held.data.ekr_version,
                seed_digest: held.data.seed_digest,
                state: held.state,
            })
            .collect())
    }
}

/// `cortex.instance.Sources`, generated: every row is one the specification fully determines from the stored `cortex.instance.Source`s.
impl<P> crate::instance::obligations::SourcesQuery for Generated<P>
where
    P: SourceStorage,
{
    fn sources(&self) -> Result<Vec<crate::instance::Sources>, UnmetObligation> {
        let admitted = SourceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::instance::Sources {
                source_id: held.data.source_id,
                instance_name: held.data.instance_name,
                name: held.data.name,
                kind: held.data.kind,
                schedule: held.data.schedule,
                runs: held.data.runs,
                consecutive_failures: held.data.consecutive_failures,
                state: held.state,
                last_outcome: held.data.last_outcome,
                last_run_at: held.data.last_run_at,
                last_reason: held.data.last_reason,
            })
            .collect())
    }
}

/// The typed refusal of a request the model declares no outcome for.
fn undeclared(source: &'static str) -> UnmetObligation {
    let capability = "command behaviour";
    UnmetObligation { capability, source }
}

/// A guard's truth, where it has one: Unknown selects no branch, so the model declares no outcome.
fn decided(truth: Option<bool>, command: &'static str) -> Result<bool, UnmetObligation> {
    truth.ok_or_else(|| undeclared(command))
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}

// generated from cortex v1
// model digest 7d8468225c0615bc54d37ab20d8a5ed011c6af620b27b8f62299e46196457ec1
// contract digest e28cd1562bab08223ec6f65f42a29bccbb41a2ebb899f233ef6abd757672022d
// do not edit: regenerate with `ess synthesize --layout crate`

//! Answers — `cortex.ask`.
//!
//! The answer service of an instance: `cortex ask serve` answers a question over the instance's store as a stream of events. It reads the store only through the EKR MCP reader (`ekr mcp`) and writes nothing to it; its model is the instance's model gateway, spoken to as an OpenAI-compatible endpoint. Every answer is streamed and kept as an answer record, with the votes of its readers; an answer voted down, failed or slow is queued for review. The records are cortex's own, never in the store: in `ask.sqlite` in the instance directory on a `sqlite` store, in a PostgreSQL schema of their own on a `postgres` one; they are purged after the spec's `retention_days`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// AnswerId — `cortex.ask.AnswerId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerId(pub String);

/// AnswerOutcome — `cortex.ask.AnswerOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// `answered`.
    Answered,
    /// `failed`.
    Failed,
    /// `limited`.
    Limited,
}

/// The states of `cortex.ask.AnswerRecord`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `AnswerRecord<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerRecordState {
    /// `Kept`.
    Kept,
}

/// AskDelta — `cortex.ask.AskDelta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDelta {
    /// `text` — `String`.
    pub text: String,
}

/// AskDiscard — `cortex.ask.AskDiscard`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDiscard {
    /// `step` — `Integer`.
    pub step: i64,
}

/// AskDone — `cortex.ask.AskDone`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDone {
    /// `answer_id` — `String`.
    pub answer_id: String,
    /// `answer` — `String`.
    pub answer: String,
    /// `nodes` — `List<String>`.
    pub nodes: Vec<String>,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `input_tokens` — `Integer`.
    pub input_tokens: i64,
    /// `output_tokens` — `Integer`.
    pub output_tokens: i64,
    /// `cached_tokens` — `Integer`.
    pub cached_tokens: i64,
    /// `context_tokens` — `Integer`.
    pub context_tokens: i64,
    /// `steps` — `Integer`.
    pub steps: i64,
    /// `unrestored` — `Integer`.
    pub unrestored: i64,
}

/// AskError — `cortex.ask.AskError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskError {
    /// `reason` — `String`.
    pub reason: String,
    /// `message` — `String`.
    pub message: String,
    /// `retry_after` — `Optional<Integer>`.
    pub retry_after: Option<i64>,
}

/// AskEvent — `cortex.ask.AskEvent`: one of a fixed set of shapes, tagged on the wire by `event`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskEvent {
    /// Tagged `delta` — `cortex.ask.AskDelta`.
    Delta(AskDelta),
    /// Tagged `discard` — `cortex.ask.AskDiscard`.
    Discard(AskDiscard),
    /// Tagged `done` — `cortex.ask.AskDone`.
    Done(AskDone),
    /// Tagged `error` — `cortex.ask.AskError`.
    Error(AskError),
    /// Tagged `status` — `cortex.ask.AskStatus`.
    Status(AskStatus),
    /// Tagged `tool` — `cortex.ask.AskTool`.
    Tool(AskTool),
}

/// AskLimits — `cortex.ask.AskLimits`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskLimits {
    /// `max_steps` — `Integer`.
    pub max_steps: i64,
    /// `max_result_bytes` — `Integer`.
    pub max_result_bytes: i64,
    /// `max_seconds` — `Integer`.
    pub max_seconds: i64,
    /// `per_identity_per_minute` — `Integer`.
    pub per_identity_per_minute: i64,
}

/// AskSpec — `cortex.ask.AskSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskSpec {
    /// `listen` — `String`.
    pub listen: String,
    /// `identity_header` — `String`.
    pub identity_header: String,
    /// `limits` — `cortex.ask.AskLimits`.
    pub limits: AskLimits,
    /// `link_templates` — `Optional<cortex.ask.LinkTemplates>`.
    pub link_templates: Option<LinkTemplates>,
    /// `guidance` — `Optional<String>`.
    pub guidance: Option<String>,
    /// `retention_days` — `Optional<Integer>`.
    pub retention_days: Option<i64>,
    /// `slow_after_ms` — `Optional<Integer>`.
    pub slow_after_ms: Option<i64>,
    /// `records` — `Optional<cortex.ask.PostgresRecords>`.
    pub records: Option<PostgresRecords>,
}

/// AskStatus — `cortex.ask.AskStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskStatus {
    /// `format` — `String`.
    pub format: String,
    /// `step` — `Integer`.
    pub step: i64,
    /// `message` — `String`.
    pub message: String,
}

/// AskTool — `cortex.ask.AskTool`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskTool {
    /// `step` — `Integer`.
    pub step: i64,
    /// `name` — `String`.
    pub name: String,
    /// `arguments` — `String`.
    pub arguments: String,
    /// `result_bytes` — `Integer`.
    pub result_bytes: i64,
    /// `truncated` — `Boolean`.
    pub truncated: bool,
    /// `failed` — `Boolean`.
    pub failed: bool,
    /// `withheld` — `Boolean`.
    pub withheld: bool,
}

/// LinkTemplates — `cortex.ask.LinkTemplates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTemplates {
    /// `node` — `Optional<String>`.
    pub node: Option<String>,
    /// `evidence` — `Optional<String>`.
    pub evidence: Option<String>,
}

/// PostgresRecords — `cortex.ask.PostgresRecords`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresRecords {
    /// `url` — `String`.
    pub url: String,
    /// `schema` — `String`.
    pub schema: String,
    /// `ca_file` — `Optional<String>`.
    pub ca_file: Option<String>,
    /// `connection` — `cortex.instance.ConnectionRef`.
    pub connection: crate::instance::ConnectionRef,
}

/// ReviewCause — `cortex.ask.ReviewCause`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewCause {
    /// `voted-down`.
    VotedDown,
    /// `failed`.
    Failed,
    /// `slow`.
    Slow,
}

/// ReviewId — `cortex.ask.ReviewId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewId(pub String);

/// The states of `cortex.ask.ReviewItem`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ReviewItem<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewItemState {
    /// `Closed`.
    Closed,
    /// `Open`.
    Open,
}

/// The states of `cortex.ask.Vote`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Vote<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteState {
    /// `Cast`.
    Cast,
}

/// VoteId — `cortex.ask.VoteId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteId(pub String);

/// VoteValue — `cortex.ask.VoteValue`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteValue {
    /// `up`.
    Up,
    /// `down`.
    Down,
}

/// What AnswerRecord — `cortex.ask.AnswerRecord` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`AnswerRecord<S>`], and at a boundary by [`AnswerRecordSnapshot::state`].
///
/// Every value satisfies `latency_ms >= 0` — checked by [`AnswerRecordData::broken_invariant`].
/// Every value satisfies `input_tokens >= 0` — checked by [`AnswerRecordData::broken_invariant`].
/// Every value satisfies `output_tokens >= 0` — checked by [`AnswerRecordData::broken_invariant`].
/// Every value satisfies `cached_tokens >= 0` — checked by [`AnswerRecordData::broken_invariant`].
/// Every value satisfies `context_tokens >= 0` — checked by [`AnswerRecordData::broken_invariant`].
/// Every value satisfies `steps >= 0` — checked by [`AnswerRecordData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRecordData {
    /// The identity: `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `instance_name` — `cortex.instance.InstanceName`.
    ///
    /// Carries `answers`: `cortex.instance.Instance` owns many `cortex.ask.AnswerRecord`.
    pub instance_name: crate::instance::InstanceName,
    /// `identity` — `String`.
    pub identity: String,
    /// `question` — `String`.
    pub question: String,
    /// `answer` — `Optional<String>`.
    pub answer: Option<String>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
    /// `outcome` — `cortex.ask.AnswerOutcome`.
    pub outcome: AnswerOutcome,
    /// `tool_calls` — `List<cortex.ask.AskTool>`.
    pub tool_calls: Vec<AskTool>,
    /// `latency_ms` — `Integer`.
    pub latency_ms: i64,
    /// `input_tokens` — `Integer`.
    pub input_tokens: i64,
    /// `output_tokens` — `Integer`.
    pub output_tokens: i64,
    /// `cached_tokens` — `Integer`.
    pub cached_tokens: i64,
    /// `context_tokens` — `Integer`.
    pub context_tokens: i64,
    /// `steps` — `Integer`.
    pub steps: i64,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

impl AnswerRecordData {
    /// The first declared invariant of `cortex.ask.AnswerRecord` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.latency_ms)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("latency_ms >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.input_tokens)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("input_tokens >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.output_tokens)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("output_tokens >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.cached_tokens)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("cached_tokens >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.context_tokens)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("context_tokens >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.steps)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("steps >= 0");
        }
        None
    }
}

/// The states of `cortex.ask.AnswerRecord`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](answer_record_state::Marker), so [`AnswerRecord<S>`](AnswerRecord) can only ever rest in a real state.
pub mod answer_record_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Kept {}
    }

    /// A declared state of `AnswerRecord`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AnswerRecordState;
    }

    /// `Kept`. Where a new instance starts.
    pub struct Kept;

    impl Marker for Kept {
        const STATE: super::AnswerRecordState = super::AnswerRecordState::Kept;
    }
}

/// AnswerRecord — `cortex.ask.AnswerRecord` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Kept`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AnswerRecordSnapshot`]
/// and [`AnswerRecordSnapshot::refine`].
pub struct AnswerRecord<S: answer_record_state::Marker> {
    data: AnswerRecordData,
    state: core::marker::PhantomData<S>,
}

impl<S: answer_record_state::Marker> AnswerRecord<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AnswerRecordState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AnswerRecordData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AnswerRecordData {
        self.data
    }
}

impl AnswerRecord<answer_record_state::Kept> {
    /// A new instance, resting in `Kept` — the only state the lifecycle starts one in.
    pub fn new(data: AnswerRecordData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.ask.AnswerRecord` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AnswerRecordSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRecordSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AnswerRecordState,
    /// What it holds.
    pub data: AnswerRecordData,
}

/// An `AnswerRecord` in whichever declared state it was found.
pub enum AnyAnswerRecord {
    /// Resting in `Kept`.
    Kept(AnswerRecord<answer_record_state::Kept>),
}

impl AnswerRecordSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AnswerRecordState` cannot spell one.
    pub fn refine(self) -> AnyAnswerRecord {
        match self.state {
            AnswerRecordState::Kept => AnyAnswerRecord::Kept(AnswerRecord {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAnswerRecord {
    /// The state, as the runtime value.
    pub fn state(&self) -> AnswerRecordState {
        match self {
            Self::Kept(_) => AnswerRecordState::Kept,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AnswerRecordSnapshot {
        match self {
            Self::Kept(instance) => AnswerRecordSnapshot {
                state: AnswerRecordState::Kept,
                data: instance.into_data(),
            },
        }
    }
}

/// What ReviewItem — `cortex.ask.ReviewItem` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ReviewItem<S>`], and at a boundary by [`ReviewItemSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItemData {
    /// The identity: `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    ///
    /// Carries `reviews`: `cortex.ask.AnswerRecord` owns many `cortex.ask.ReviewItem`.
    pub answer_id: AnswerId,
    /// `cause` — `cortex.ask.ReviewCause`.
    pub cause: ReviewCause,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `close_reason` — `Optional<String>`.
    pub close_reason: Option<String>,
    /// `closed_at` — `Optional<Timestamp>`.
    pub closed_at: Option<crate::primitives::Timestamp>,
    /// `expect_facts` — `Optional<List<String>>`.
    pub expect_facts: Option<Vec<String>>,
    /// `expect_nodes` — `Optional<List<String>>`.
    pub expect_nodes: Option<Vec<String>>,
}

/// The states of `cortex.ask.ReviewItem`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](review_item_state::Marker), so [`ReviewItem<S>`](ReviewItem) can only ever rest in a real state.
pub mod review_item_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `ReviewItem`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ReviewItemState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::ReviewItemState = super::ReviewItemState::Closed;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::ReviewItemState = super::ReviewItemState::Open;
    }
}

/// ReviewItem — `cortex.ask.ReviewItem` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ReviewItemSnapshot`]
/// and [`ReviewItemSnapshot::refine`].
pub struct ReviewItem<S: review_item_state::Marker> {
    data: ReviewItemData,
    state: core::marker::PhantomData<S>,
}

impl<S: review_item_state::Marker> ReviewItem<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ReviewItemState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ReviewItemData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ReviewItemData {
        self.data
    }
}

impl ReviewItem<review_item_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: ReviewItemData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl ReviewItem<review_item_state::Open> {
    /// `close` — `Open` → `Closed`. Taken by the `closed` outcome of `cortex.ask.CloseReview`.
    pub fn close(self) -> ReviewItem<review_item_state::Closed> {
        ReviewItem {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.ask.ReviewItem` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ReviewItemSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItemSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ReviewItemState,
    /// What it holds.
    pub data: ReviewItemData,
}

/// An `ReviewItem` in whichever declared state it was found.
pub enum AnyReviewItem {
    /// Resting in `Closed`.
    Closed(ReviewItem<review_item_state::Closed>),
    /// Resting in `Open`.
    Open(ReviewItem<review_item_state::Open>),
}

impl ReviewItemSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ReviewItemState` cannot spell one.
    pub fn refine(self) -> AnyReviewItem {
        match self.state {
            ReviewItemState::Closed => AnyReviewItem::Closed(ReviewItem {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ReviewItemState::Open => AnyReviewItem::Open(ReviewItem {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyReviewItem {
    /// The state, as the runtime value.
    pub fn state(&self) -> ReviewItemState {
        match self {
            Self::Closed(_) => ReviewItemState::Closed,
            Self::Open(_) => ReviewItemState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ReviewItemSnapshot {
        match self {
            Self::Closed(instance) => ReviewItemSnapshot {
                state: ReviewItemState::Closed,
                data: instance.into_data(),
            },
            Self::Open(instance) => ReviewItemSnapshot {
                state: ReviewItemState::Open,
                data: instance.into_data(),
            },
        }
    }
}

/// What Vote — `cortex.ask.Vote` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Vote<S>`], and at a boundary by [`VoteSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteData {
    /// The identity: `vote_id` — `cortex.ask.VoteId`.
    pub vote_id: VoteId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    ///
    /// Carries `votes`: `cortex.ask.AnswerRecord` owns many `cortex.ask.Vote`.
    pub answer_id: AnswerId,
    /// `identity` — `String`.
    pub identity: String,
    /// `value` — `cortex.ask.VoteValue`.
    pub value: VoteValue,
    /// `comment` — `Optional<String>`.
    pub comment: Option<String>,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// The states of `cortex.ask.Vote`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](vote_state::Marker), so [`Vote<S>`](Vote) can only ever rest in a real state.
pub mod vote_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Cast {}
    }

    /// A declared state of `Vote`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::VoteState;
    }

    /// `Cast`. Where a new instance starts.
    pub struct Cast;

    impl Marker for Cast {
        const STATE: super::VoteState = super::VoteState::Cast;
    }
}

/// Vote — `cortex.ask.Vote` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Cast`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`VoteSnapshot`]
/// and [`VoteSnapshot::refine`].
pub struct Vote<S: vote_state::Marker> {
    data: VoteData,
    state: core::marker::PhantomData<S>,
}

impl<S: vote_state::Marker> Vote<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> VoteState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &VoteData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> VoteData {
        self.data
    }
}

impl Vote<vote_state::Cast> {
    /// A new instance, resting in `Cast` — the only state the lifecycle starts one in.
    pub fn new(data: VoteData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.ask.Vote` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`VoteSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: VoteState,
    /// What it holds.
    pub data: VoteData,
}

/// An `Vote` in whichever declared state it was found.
pub enum AnyVote {
    /// Resting in `Cast`.
    Cast(Vote<vote_state::Cast>),
}

impl VoteSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `VoteState` cannot spell one.
    pub fn refine(self) -> AnyVote {
        match self.state {
            VoteState::Cast => AnyVote::Cast(Vote {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyVote {
    /// The state, as the runtime value.
    pub fn state(&self) -> VoteState {
        match self {
            Self::Cast(_) => VoteState::Cast,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> VoteSnapshot {
        match self {
            Self::Cast(instance) => VoteSnapshot {
                state: VoteState::Cast,
                data: instance.into_data(),
            },
        }
    }
}

/// Ask a question — the input of `cortex.ask.Ask`.
///
/// Everything it can result in is [`AskOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// `question` — `String`.
    pub question: String,
    /// `identity` — `String`.
    pub identity: String,
}

/// Everything `cortex.ask.Ask` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskOutcome {
    /// `unidentified` — externally decided (The request carries no identity in the header the spec names).
    ///
    /// It answers 401; no model was asked.
    Unidentified {
        /// Why it was refused: `cortex.ask.Unidentified`.
        error: Unidentified,
    },
    /// `rate-limited` — externally decided (The identity asked its per-minute limit of questions in the last minute).
    ///
    /// It answers 429 with an `error` event carrying `retry_after`; no model was asked.
    RateLimited {
        /// Why it was refused: `cortex.ask.RateLimited`.
        error: RateLimited,
    },
    /// `limit-reached` — externally decided (The model still asks for a tool at the step limit, or the time limit passes).
    ///
    /// The stream ends with an `error` event naming the limit, and no `done`.
    LimitReached {
        /// Why it was refused: `cortex.ask.LimitReached`.
        error: LimitReached,
    },
    /// `model-failed` — externally decided (The model gateway cannot be reached or answers an error).
    ///
    /// The stream ends with an `error` event, and no `done`.
    ModelFailed {
        /// Why it was refused: `cortex.ask.ModelFailed`.
        error: ModelFailed,
    },
    /// `reader-failed` — externally decided (The EKR MCP reader cannot be started or stops answering).
    ///
    /// The stream ends with an `error` event, and no `done`.
    ReaderFailed {
        /// Why it was refused: `cortex.ask.ReaderFailed`.
        error: ReaderFailed,
    },
    /// `answered` — otherwise.
    ///
    /// The stream ends with a `done` event carrying the answer and what it cites.
    Answered {
        /// The `cortex.ask.Answered` this outcome publishes.
        answered: Answered,
    },
}

/// Vote on an answer — the input of `cortex.ask.CastVote`.
///
/// Everything it can result in is [`CastVoteOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastVote {
    /// `vote_id` — `cortex.ask.VoteId`.
    pub vote_id: VoteId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `identity` — `String`.
    pub identity: String,
    /// `value` — `cortex.ask.VoteValue`.
    pub value: VoteValue,
    /// `comment` — `Optional<String>`.
    pub comment: Option<String>,
}

/// Everything `cortex.ask.CastVote` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastVoteOutcome {
    /// `unidentified` — externally decided (The request carries no identity in the header the spec names).
    ///
    /// It answers 401; nothing was cast.
    Unidentified {
        /// Why it was refused: `cortex.ask.Unidentified`.
        error: Unidentified,
    },
    /// `no-such-answer` — externally decided (No answer record carries `answer_id`).
    ///
    /// It answers 404; nothing was cast.
    NoSuchAnswer {
        /// Why it was refused: `cortex.ask.AnswerNotFound`.
        error: AnswerNotFound,
    },
    /// `changed` — otherwise.
    ///
    /// The identity's vote on the answer is replaced; it still has one.
    Changed {
        /// The `cortex.ask.Voted` this outcome publishes.
        voted: Voted,
    },
    /// `cast` — for an identity no record carries.
    ///
    /// The identity's first vote on the answer is kept.
    Cast {
        /// The `cortex.ask.Voted` this outcome publishes.
        voted: Voted,
    },
}

/// Close a review item — the input of `cortex.ask.CloseReview`.
///
/// Everything it can result in is [`CloseReviewOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseReview {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `reason` — `String`.
    pub reason: String,
    /// `expect_facts` — `List<String>`.
    pub expect_facts: Vec<String>,
    /// `expect_nodes` — `List<String>`.
    pub expect_nodes: Vec<String>,
}

/// Everything `cortex.ask.CloseReview` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReviewOutcome {
    /// `closed` — otherwise.
    ///
    /// The item leaves the review queue with its reason.
    Closed {
        /// The `cortex.ask.ReviewClosed` this outcome publishes.
        review_closed: ReviewClosed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// It answers 409; the item keeps the reason it was closed with.
    WrongState {
        /// Why it was refused: `cortex.ask.ReviewNotOpen`.
        error: ReviewNotOpen,
    },
    /// `no-such-review` — for an identity no record carries.
    ///
    /// It answers 404; nothing changed.
    NoSuchReview {
        /// Why it was refused: `cortex.ask.ReviewNotFound`.
        error: ReviewNotFound,
    },
}

/// Open a review item — the input of `cortex.ask.OpenReview`.
///
/// Everything it can result in is [`OpenReviewOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReview {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `cause` — `cortex.ask.ReviewCause`.
    pub cause: ReviewCause,
}

/// Everything `cortex.ask.OpenReview` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenReviewOutcome {
    /// `opened` — otherwise.
    ///
    /// The answer is in the review queue for this cause.
    Opened {
        /// The `cortex.ask.ReviewOpened` this outcome publishes.
        review_opened: ReviewOpened,
    },
    /// `exists` — for an identity a record already carries.
    ///
    /// The answer already has an item for this cause; nothing changed.
    Exists {
        /// Why it was refused: `cortex.ask.ReviewExists`.
        error: ReviewExists,
    },
}

/// Purge expired answers — the input of `cortex.ask.PurgeExpired`.
///
/// Everything it can result in is [`PurgeExpiredOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeExpired {
    /// `before` — `Timestamp`.
    pub before: crate::primitives::Timestamp,
}

/// Everything `cortex.ask.PurgeExpired` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurgeExpiredOutcome {
    /// `purged` — otherwise.
    ///
    /// Nothing older than `before` is left.
    Purged {
        /// The `cortex.ask.Purged` this outcome publishes.
        purged: Purged,
    },
}

/// Record an answer — the input of `cortex.ask.RecordAnswer`.
///
/// Everything it can result in is [`RecordAnswerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAnswer {
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `instance_name` — `cortex.instance.InstanceName`.
    pub instance_name: crate::instance::InstanceName,
    /// `identity` — `String`.
    pub identity: String,
    /// `question` — `String`.
    pub question: String,
    /// `answer` — `Optional<String>`.
    pub answer: Option<String>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
    /// `outcome` — `cortex.ask.AnswerOutcome`.
    pub outcome: AnswerOutcome,
    /// `tool_calls` — `List<cortex.ask.AskTool>`.
    pub tool_calls: Vec<AskTool>,
    /// `latency_ms` — `Integer`.
    pub latency_ms: i64,
    /// `input_tokens` — `Integer`.
    pub input_tokens: i64,
    /// `output_tokens` — `Integer`.
    pub output_tokens: i64,
    /// `cached_tokens` — `Integer`.
    pub cached_tokens: i64,
    /// `context_tokens` — `Integer`.
    pub context_tokens: i64,
    /// `steps` — `Integer`.
    pub steps: i64,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `cortex.ask.RecordAnswer` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordAnswerOutcome {
    /// `recorded` — otherwise.
    ///
    /// The answer is kept until it is past `retention_days`.
    Recorded {
        /// The `cortex.ask.AnswerRecorded` this outcome publishes.
        answer_recorded: AnswerRecorded,
    },
}

/// AnswerRecorded — the event `cortex.ask.AnswerRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRecorded {
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `outcome` — `cortex.ask.AnswerOutcome`.
    pub outcome: AnswerOutcome,
}

/// Answered — the event `cortex.ask.Answered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    /// `identity` — `String`.
    pub identity: String,
    /// `done` — `cortex.ask.AskDone`.
    pub done: AskDone,
}

/// Purged — the event `cortex.ask.Purged`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purged {
    /// `before` — `Timestamp`.
    pub before: crate::primitives::Timestamp,
    /// `records` — `Integer`.
    pub records: i64,
    /// `votes` — `Integer`.
    pub votes: i64,
    /// `reviews` — `Integer`.
    pub reviews: i64,
}

/// ReviewClosed — the event `cortex.ask.ReviewClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewClosed {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `reason` — `String`.
    pub reason: String,
}

/// ReviewOpened — the event `cortex.ask.ReviewOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOpened {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `cause` — `cortex.ask.ReviewCause`.
    pub cause: ReviewCause,
}

/// Voted — the event `cortex.ask.Voted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voted {
    /// `vote_id` — `cortex.ask.VoteId`.
    pub vote_id: VoteId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `value` — `cortex.ask.VoteValue`.
    pub value: VoteValue,
}

/// The declared error `cortex.ask.AnswerNotFound`.
///
/// No answer record carries this id, or it was purged; it answers 404.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerNotFound {
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
}

/// The declared error `cortex.ask.LimitReached`.
///
/// The answer reached the spec's step or time limit before the model answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitReached {
    /// `limit` — `String`.
    pub limit: String,
}

/// The declared error `cortex.ask.ModelFailed`.
///
/// The model gateway could not be reached or answered an error or no message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.ask.RateLimited`.
///
/// The identity asked `per_identity_per_minute` questions in the last minute; it answers 429. Ask again after `retry_after` seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimited {
    /// `identity` — `String`.
    pub identity: String,
    /// `retry_after` — `Integer`.
    pub retry_after: i64,
}

/// The declared error `cortex.ask.ReaderFailed`.
///
/// The EKR MCP reader could not be started or stopped answering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.ask.ReviewExists`.
///
/// The answer already has a review item for this cause, open or closed; the service opens no second one and answers nothing for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewExists {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
}

/// The declared error `cortex.ask.ReviewNotFound`.
///
/// No review item carries this id, or it was purged; it answers 404.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewNotFound {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
}

/// The declared error `cortex.ask.ReviewNotOpen`.
///
/// The review item was already closed; it answers 409 and keeps its first reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewNotOpen {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
}

/// The declared error `cortex.ask.Unidentified`.
///
/// The request carries no identity in the header the spec names; it answers 401.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unidentified {
    /// `header` — `String`.
    pub header: String,
}

/// Open reviews — one row of the view `cortex.ask.OpenReviews`.
///
/// Projects `cortex.ask.ReviewItem` at `read_your_writes` consistency, containing instances where `state == Open`.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReviews {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: ReviewId,
    /// `answer_id` — `cortex.ask.AnswerId`.
    pub answer_id: AnswerId,
    /// `cause` — `cortex.ask.ReviewCause`.
    pub cause: ReviewCause,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `cortex.ask.Ask` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AskBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.Ask`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn ask(&mut self, input: super::Ask) -> Result<super::AskOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.ask.CastVote` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CastVoteBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.CastVote`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn cast_vote(&mut self, input: super::CastVote) -> Result<super::CastVoteOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.ask.CloseReview` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CloseReviewBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.CloseReview`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn close_review(&mut self, input: super::CloseReview) -> Result<super::CloseReviewOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.ask.OpenReview` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenReviewBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.OpenReview`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_review(&mut self, input: super::OpenReview) -> Result<super::OpenReviewOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.ask.PurgeExpired` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PurgeExpiredBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.PurgeExpired`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn purge_expired(&mut self, input: super::PurgeExpired) -> Result<super::PurgeExpiredOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.ask.RecordAnswer` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordAnswerBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.ask.RecordAnswer`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_answer(&mut self, input: super::RecordAnswer) -> Result<super::RecordAnswerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `cortex.ask.OpenReviews` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by an order over the `Timestamp` field `opened_at`, which the suite ranks by its instant and a generated query does not.
    ///
    /// Contract: a query answering `cortex.ask.OpenReviews` with rows projected from `cortex.ask.ReviewItem` at `read_your_writes` consistency, containing instances where `state == Open`.
    pub trait OpenReviewsQuery {
        /// Serves `cortex.ask.OpenReviews` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn open_reviews(&self) -> Result<Vec<super::OpenReviews>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl OpenReviewsQuery for Unimplemented {
        fn open_reviews(&self) -> Result<Vec<super::OpenReviews>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "cortex.ask.OpenReviews" })
        }
    }
}

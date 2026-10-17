// generated from cortex v1
// model digest 5b8d449a9b5dd33fe4da24d7b0507f49b8bbbd969b3c9007942d6f34fea63878
// contract digest 2f5be3eec0297f1a905f33f1e2355526c36078813e7f121ca0bb406b9b6ef9be
// do not edit: regenerate with `ess synthesize --layout crate`

//! Answers — `cortex.ask`.
//!
//! The answer service of an instance: `cortex ask serve` answers a question over the instance's store as a stream of events. It reads the store only through the EKR MCP reader (`ekr mcp`) and writes nothing to it; its model is the instance's model gateway, spoken to as an OpenAI-compatible endpoint. An answer is streamed, not stored.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

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

/// Answered — the event `cortex.ask.Answered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    /// `identity` — `String`.
    pub identity: String,
    /// `done` — `cortex.ask.AskDone`.
    pub done: AskDone,
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

/// The declared error `cortex.ask.Unidentified`.
///
/// The request carries no identity in the header the spec names; it answers 401.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unidentified {
    /// `header` — `String`.
    pub header: String,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
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

}

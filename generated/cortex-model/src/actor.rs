// generated from cortex v1
// model digest 3cb764fa6058c327300de7d63b53b82b3ed86189c61ff97550848ad850d64bcf
// contract digest 5daf559cd61a381fbf6ce9233bce2e7d477c4713ee671105b36029ab93dc414a
// do not edit: regenerate with `ess synthesize --layout crate`

//! Every actor the specification declares, and the commands each may invoke — as data.
//!
//! A grant is checked against a caller identity, which these types do not read from anywhere:
//! whatever authenticates a request builds a [`Caller`], and a served surface checks it against
//! [`may`] before the command runs. The `PLAN.md` beside this workspace says, per actor,
//! whether a generated surface enforces the grant or the caller does.

/// An actor the specification declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Actor {
    /// `cortex.ask.Asker`.
    Asker,
    /// `cortex.ask.Reviewer`.
    Reviewer,
    /// `cortex.ask.Service`.
    Service,
    /// `cortex.instance.Operator`.
    Operator,
    /// `cortex.instance.Scheduler`.
    Scheduler,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::Asker,
        Actor::Reviewer,
        Actor::Service,
        Actor::Operator,
        Actor::Scheduler,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::Asker => "cortex.ask.Asker",
            Actor::Reviewer => "cortex.ask.Reviewer",
            Actor::Service => "cortex.ask.Service",
            Actor::Operator => "cortex.instance.Operator",
            Actor::Scheduler => "cortex.instance.Scheduler",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::Asker => &[
            "cortex.ask.Ask",
            "cortex.ask.CastVote",
        ],
        Actor::Reviewer => &[
            "cortex.ask.CloseReview",
        ],
        Actor::Service => &[
            "cortex.ask.OpenReview",
            "cortex.ask.PurgeExpired",
            "cortex.ask.RecordAnswer",
        ],
        Actor::Operator => &[
            "cortex.instance.AddEvalCase",
            "cortex.instance.AddSource",
            "cortex.instance.AdoptInstance",
            "cortex.instance.ApplySchemaProposals",
            "cortex.instance.ApproveSchemaProposals",
            "cortex.instance.CreateInstance",
            "cortex.instance.EnableSource",
            "cortex.instance.MeasureQuality",
            "cortex.instance.ProposeSchemaChanges",
            "cortex.instance.RejectSchemaProposals",
            "cortex.instance.RemoveInstance",
            "cortex.instance.RestoreSnapshot",
            "cortex.instance.RunEval",
            "cortex.instance.UpdateInstance",
        ],
        Actor::Scheduler => &[
            "cortex.instance.RecordFailure",
            "cortex.instance.RunSource",
        ],
    }
}

/// Who a request was authenticated as.
///
/// Built by whatever authenticates the request — a session, a token, a certificate — and
/// handed to the served surface's `dispatch` and `handle`, which check its grant
/// before the command runs. Never derived from the request itself: a client can write
/// anything into a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caller {
    /// The declared actor.
    pub actor: Actor,
}

impl Caller {
    /// `true` when this caller may invoke `command`, named by its qualified name.
    pub fn may(&self, command: &str) -> bool {
        may(self.actor).contains(&command)
    }
}

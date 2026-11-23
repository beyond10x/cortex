// generated from cortex v1
// model digest 37a5c9c7902bf753546e91cac2e2cdfa93819d7b3efc21c30615b90c15f60700
// contract digest 76c02a54a9220bcadae5fc0cb0a45641bbaa6daab25a052e0281bb4dd744a26c
// do not edit: regenerate with `ess synthesize --layout crate`

//! The typed refusal of an unmet obligation, and the conversion seams owed between contexts.
//!
//! An obligation is a capability the synthesis plan owes the implementor — the contract is declared,
//! the behaviour is not. Until an implementation satisfies one, its stub returns [`UnmetObligation`]:
//! a value naming the plan entry, never a panic and never a guess, so a workspace built on stubs
//! compiles and reports its own gaps.

/// A capability the synthesis plan owes and nothing has satisfied yet.
///
/// The two fields spell the plan entry: look the pair up in `PLAN.md` for the contract being
/// refused. A satisfying implementation never constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetObligation {
    /// The capability kind, as the plan spells it.
    pub capability: &'static str,
    /// The construct that requires it, in the specification's own spelling.
    pub source: &'static str,
}

impl core::fmt::Display for UnmetObligation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unmet obligation: {} `{}` — see PLAN.md", self.capability, self.source)
    }
}

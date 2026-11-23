// generated from cortex v1
// model digest bbff72fdf900b1993ba797c3c116b8627873447cfe6db020eb0e539995aa8356
// contract digest 52028c2600ca4111deee01b6907aa2b6dee9a0857cea71675373ed9f85cdc737
// do not edit: regenerate with `ess synthesize --layout crate`

//! Semantic types synthesised from the `cortex` specification, v1.
//!
//! Spin up knowledge instances from a spec. Each instance is one EKR store fed on a schedule by the data sources its spec connects: web pages found by search or crawled from listed sites, records of any Connectors operation, and local files. Secrets never pass through cortex; a source names a Connectors connection and only invokes operations through it.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actor;
pub mod ask;
pub mod behaviour;
pub mod instance;
pub mod json;
pub mod obligation;
pub mod primitives;

pub mod ports;
pub mod system;

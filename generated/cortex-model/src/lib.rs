// generated from cortex v1
// model digest 4f67d485d3f511f1486cfde0aa63dbc01b243139aeea5c2e9814ae5593be08b5
// contract digest 7dcae4b5491fdc744c4a216c888e2e5adb0b0d6dd2e6c0197059588209ac8cd8
// do not edit: regenerate with `ess synthesize --layout crate`

//! Semantic types synthesised from the `cortex` specification, v1.
//!
//! Spin up knowledge brains from a spec. Each instance is one EKR store fed on a schedule by the data sources its spec connects: web pages found by search or crawled from listed sites, records of any Connectors operation, and local files. Secrets never pass through cortex; a source names a Connectors connection and only invokes operations through it.
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
pub mod behaviour;
pub mod instance;
pub mod json;
pub mod obligation;
pub mod primitives;

pub mod ports;
pub mod system;

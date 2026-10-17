//! The answer records of `cortex ask serve` (`cortex.ask.AnswerRecord`, `cortex.ask.Vote`,
//! `cortex.ask.ReviewItem`): cortex's own tables, never the store's.
//!
//! Storage is one seam, [`RecordStore`]; [`Records`] is its SQLite backend, `ask.sqlite` in the
//! instance directory, with a schema version and a migration on open. [`postgres::PgRecords`] is
//! its PostgreSQL backend, for an instance on a `postgres` store: a schema of its own named by
//! `ask.records`, reached through a records process that a Connectors connection of its own
//! launches, so the service never holds the password.
//!
//! [`Keeper`] is what the service does with the records: it runs the generated behaviours of
//! `RecordAnswer`, `CastVote`, `OpenReview`, `CloseReview` and `PurgeExpired` over the store
//! (`crate::ports::AskPorts`), deciding each `external:` branch from the store first.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use cortex_model::ask as ma;
use cortex_model::ask::obligations::{
    CastVoteBehavior, CloseReviewBehavior, OpenReviewBehavior, OpenReviewsQuery,
    PurgeExpiredBehavior, RecordAnswerBehavior,
};
use cortex_model::behaviour::Generated;
use cortex_model::instance as m;
use cortex_model::obligation::UnmetObligation;
use cortex_model::primitives::Timestamp;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::home::Home;
use crate::ports::{AskPorts, Shared};

pub mod postgres;

/// The file the records of an instance on a SQLite store live in, in its instance directory.
pub const FILE: &str = "ask.sqlite";

/// How many days a record is kept when the spec names no `ask.retention_days`.
pub const DEFAULT_RETENTION_DAYS: i64 = 90;

/// What a purge deleted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Purge {
    pub records: i64,
    pub votes: i64,
    pub reviews: i64,
}

/// The `Err` of [`RecordStore::put_vote`] and [`RecordStore::put_review`] when the answer the vote
/// or the item belongs to is not kept (a purge took it after the command read it): nothing was
/// written.
pub const NO_SUCH_ANSWER: &str = "the answer is not kept";

/// The `Err` of [`RecordStore::put_review`] when a closed item is written over one that is no
/// longer open (another close won, or a purge took it): nothing was written.
pub const REVIEW_NOT_OPEN: &str = "the review item is not open";

/// Where the records live: the seam a backend implements. Every method is one statement or one
/// transaction of the backend; an `Err` is the backend's failure, in words, or one of
/// [`NO_SUCH_ANSWER`] and [`REVIEW_NOT_OPEN`] for a write whose condition no longer holds.
///
/// A command reads, then writes, and another request can land between the two, so each write
/// holds its own condition in the same statement: a vote and an open item are written only while
/// their answer is kept, an open item never over an existing one, and a closed item only over an
/// open one.
pub trait RecordStore: Send + Sync {
    fn answer(&self, id: &ma::AnswerId) -> Result<Option<ma::AnswerRecordSnapshot>, String>;
    fn put_answer(&self, record: &ma::AnswerRecordSnapshot) -> Result<(), String>;
    fn delete_answer(&self, id: &ma::AnswerId) -> Result<(), String>;
    fn vote(&self, id: &ma::VoteId) -> Result<Option<ma::VoteSnapshot>, String>;
    /// Casts or changes a vote while its answer is kept; [`NO_SUCH_ANSWER`] otherwise.
    fn put_vote(&self, vote: &ma::VoteSnapshot) -> Result<(), String>;
    fn delete_vote(&self, id: &ma::VoteId) -> Result<(), String>;
    fn review(&self, id: &ma::ReviewId) -> Result<Option<ma::ReviewItemSnapshot>, String>;
    /// An open item is written while its answer is kept ([`NO_SUCH_ANSWER`] otherwise) and never
    /// over an item that exists; a closed item only over an open one ([`REVIEW_NOT_OPEN`]
    /// otherwise).
    fn put_review(&self, item: &ma::ReviewItemSnapshot) -> Result<(), String>;
    fn delete_review(&self, id: &ma::ReviewId) -> Result<(), String>;
    /// Every record, oldest first.
    fn answers(&self) -> Result<Vec<ma::AnswerRecordData>, String>;
    /// The votes on one answer, oldest first.
    fn votes(&self, answer_id: &str) -> Result<Vec<ma::VoteData>, String>;
    /// The review items of one answer, open and closed, oldest first.
    fn reviews(&self, answer_id: &str) -> Result<Vec<ma::ReviewItemSnapshot>, String>;
    /// The view `cortex.ask.OpenReviews`: every open item, oldest first.
    fn open_reviews(&self) -> Result<Vec<ma::OpenReviews>, String>;
    /// Deletes every record whose `at` is before `before_ms`, with its votes and review items, and
    /// any vote or review item whose answer is gone, in one transaction.
    fn purge(&self, before_ms: i64) -> Result<Purge, String>;
}

/// The schema of `ask.sqlite`, one step per version: step `i` takes version `i` to `i + 1`, and
/// `PRAGMA user_version` holds the version a file is at. A file at a version past the last step
/// was written by a newer cortex and is refused.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE answers (
        answer_id TEXT PRIMARY KEY,
        instance_name TEXT NOT NULL,
        identity TEXT NOT NULL,
        question TEXT NOT NULL,
        answer TEXT,
        reason TEXT,
        outcome TEXT NOT NULL,
        tool_calls TEXT NOT NULL,
        latency_ms INTEGER NOT NULL,
        input_tokens INTEGER NOT NULL,
        output_tokens INTEGER NOT NULL,
        cached_tokens INTEGER NOT NULL,
        context_tokens INTEGER NOT NULL,
        steps INTEGER NOT NULL,
        at_ms INTEGER NOT NULL
    );
    CREATE INDEX answers_at ON answers (at_ms);
    CREATE TABLE votes (
        vote_id TEXT PRIMARY KEY,
        answer_id TEXT NOT NULL,
        identity TEXT NOT NULL,
        value TEXT NOT NULL,
        comment TEXT,
        at_ms INTEGER NOT NULL
    );
    CREATE INDEX votes_answer ON votes (answer_id);
    CREATE TABLE reviews (
        review_id TEXT PRIMARY KEY,
        answer_id TEXT NOT NULL,
        cause TEXT NOT NULL,
        state TEXT NOT NULL,
        opened_ms INTEGER NOT NULL,
        close_reason TEXT,
        closed_ms INTEGER
    );
    CREATE INDEX reviews_answer ON reviews (answer_id);
    CREATE INDEX reviews_state ON reviews (state, opened_ms);
"];

/// The schema version this cortex writes.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// The SQLite backend: one connection, held by one request at a time.
pub struct Records {
    db: Mutex<Connection>,
    at: PathBuf,
}

fn sql(at: &Path) -> impl Fn(rusqlite::Error) -> String + '_ {
    move |e| format!("{}: {e}", at.display())
}

impl Records {
    /// Opens `path`, creating it and its directory when missing, and migrates it to
    /// [`SCHEMA_VERSION`].
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        let db = Connection::open(path).map_err(sql(path))?;
        db.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(sql(path))?;
        Self::migrated(db, path.to_path_buf())
    }

    /// A store in memory, for tests and the conformance suite.
    pub fn in_memory() -> Result<Self, String> {
        let at = PathBuf::from(":memory:");
        let db = Connection::open_in_memory().map_err(sql(&at))?;
        Self::migrated(db, at)
    }

    fn migrated(mut db: Connection, at: PathBuf) -> Result<Self, String> {
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sql(&at))?;
        let version: i64 = tx
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(sql(&at))?;
        if version > SCHEMA_VERSION {
            return Err(format!(
                "{}: schema version {version} is newer than this cortex's {SCHEMA_VERSION}; \
                 a newer cortex wrote it",
                at.display()
            ));
        }
        for step in &MIGRATIONS[version.max(0) as usize..] {
            tx.execute_batch(step).map_err(sql(&at))?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(sql(&at))?;
        tx.commit().map_err(sql(&at))?;
        Ok(Self {
            db: Mutex::new(db),
            at,
        })
    }

    fn with<T>(&self, f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>) -> Result<T, String> {
        let mut db = self
            .db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut db).map_err(sql(&self.at))
    }

    /// Every record, oldest first.
    pub fn answers(&self) -> Result<Vec<ma::AnswerRecordData>, String> {
        let rows = self.with(|db| {
            let mut q = db.prepare(&format!("{ANSWER} ORDER BY at_ms, rowid"))?;
            let rows = q.query_map([], answer_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        rows.into_iter().collect()
    }

    /// The votes on one answer, oldest first.
    pub fn votes(&self, answer_id: &str) -> Result<Vec<ma::VoteData>, String> {
        let rows = self.with(|db| {
            let mut q = db.prepare(&format!(
                "{VOTE} WHERE answer_id = ?1 ORDER BY at_ms, rowid"
            ))?;
            let rows = q.query_map([answer_id], vote_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        rows.into_iter().map(|r| r.map(|v| v.data)).collect()
    }

    /// The review items of one answer, open and closed, oldest first.
    pub fn reviews(&self, answer_id: &str) -> Result<Vec<ma::ReviewItemSnapshot>, String> {
        let rows = self.with(|db| {
            let mut q = db.prepare(&format!(
                "{REVIEW} WHERE answer_id = ?1 ORDER BY opened_ms, rowid"
            ))?;
            let rows = q.query_map([answer_id], review_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        rows.into_iter().collect()
    }
}

const ANSWER: &str = "SELECT answer_id, instance_name, identity, question, answer, reason, \
    outcome, tool_calls, latency_ms, input_tokens, output_tokens, cached_tokens, context_tokens, \
    steps, at_ms FROM answers";
const VOTE: &str = "SELECT vote_id, answer_id, identity, value, comment, at_ms FROM votes";
const REVIEW: &str = "SELECT review_id, answer_id, cause, state, opened_ms, close_reason, \
    closed_ms FROM reviews";

/// A row of `answers`; a value the schema cannot hold (an unknown outcome, broken tool calls) is
/// the inner `Err`.
fn answer_row(r: &rusqlite::Row) -> rusqlite::Result<Result<ma::AnswerRecordData, String>> {
    let id: String = r.get(0)?;
    let outcome: String = r.get(6)?;
    let tools: String = r.get(7)?;
    let decoded = (|| {
        Ok(ma::AnswerRecordData {
            answer_id: ma::AnswerId(id.clone()),
            instance_name: m::InstanceName(r.get(1).map_err(|e| e.to_string())?),
            identity: r.get(2).map_err(|e| e.to_string())?,
            question: r.get(3).map_err(|e| e.to_string())?,
            answer: r.get(4).map_err(|e| e.to_string())?,
            reason: r.get(5).map_err(|e| e.to_string())?,
            outcome: outcome_of(&outcome).ok_or(format!("outcome {outcome:?}"))?,
            tool_calls: tools_of(&tools).ok_or("tool_calls")?,
            latency_ms: r.get(8).map_err(|e| e.to_string())?,
            input_tokens: r.get(9).map_err(|e| e.to_string())?,
            output_tokens: r.get(10).map_err(|e| e.to_string())?,
            cached_tokens: r.get(11).map_err(|e| e.to_string())?,
            context_tokens: r.get(12).map_err(|e| e.to_string())?,
            steps: r.get(13).map_err(|e| e.to_string())?,
            at: stamp(r.get(14).map_err(|e| e.to_string())?),
        })
    })();
    Ok(decoded.map_err(|e: String| format!("answer record {id}: {e}")))
}

fn vote_row(r: &rusqlite::Row) -> rusqlite::Result<Result<ma::VoteSnapshot, String>> {
    let id: String = r.get(0)?;
    let value: String = r.get(3)?;
    let Some(value) = value_of(&value) else {
        return Ok(Err(format!("vote {id}: value {value:?}")));
    };
    Ok(Ok(ma::VoteSnapshot {
        state: ma::VoteState::Cast,
        data: ma::VoteData {
            vote_id: ma::VoteId(id),
            answer_id: ma::AnswerId(r.get(1)?),
            identity: r.get(2)?,
            value,
            comment: r.get(4)?,
            at: stamp(r.get(5)?),
        },
    }))
}

fn review_row(r: &rusqlite::Row) -> rusqlite::Result<Result<ma::ReviewItemSnapshot, String>> {
    let id: String = r.get(0)?;
    let cause: String = r.get(2)?;
    let state: String = r.get(3)?;
    let (Some(cause), Some(state)) = (cause_of(&cause), review_state_of(&state)) else {
        return Ok(Err(format!(
            "review item {id}: cause {cause:?}, state {state:?}"
        )));
    };
    let closed: Option<i64> = r.get(6)?;
    Ok(Ok(ma::ReviewItemSnapshot {
        state,
        data: ma::ReviewItemData {
            review_id: ma::ReviewId(id),
            answer_id: ma::AnswerId(r.get(1)?),
            cause,
            opened_at: stamp(r.get(4)?),
            close_reason: r.get(5)?,
            closed_at: closed.map(stamp),
        },
    }))
}

impl RecordStore for Records {
    fn answer(&self, id: &ma::AnswerId) -> Result<Option<ma::AnswerRecordSnapshot>, String> {
        let row = self.with(|db| {
            db.query_row(
                &format!("{ANSWER} WHERE answer_id = ?1"),
                [&id.0],
                answer_row,
            )
            .optional()
        })?;
        row.transpose().map(|data| {
            data.map(|data| ma::AnswerRecordSnapshot {
                state: ma::AnswerRecordState::Kept,
                data,
            })
        })
    }

    fn put_answer(&self, record: &ma::AnswerRecordSnapshot) -> Result<(), String> {
        let d = &record.data;
        let at = ms_of(&d.at)?;
        self.with(|db| {
            db.execute(
                "INSERT INTO answers (answer_id, instance_name, identity, question, answer, \
                 reason, outcome, tool_calls, latency_ms, input_tokens, output_tokens, \
                 cached_tokens, context_tokens, steps, at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) \
                 ON CONFLICT (answer_id) DO UPDATE SET instance_name = ?2, identity = ?3, \
                 question = ?4, answer = ?5, reason = ?6, outcome = ?7, tool_calls = ?8, \
                 latency_ms = ?9, input_tokens = ?10, output_tokens = ?11, cached_tokens = ?12, \
                 context_tokens = ?13, steps = ?14, at_ms = ?15",
                params![
                    d.answer_id.0,
                    d.instance_name.0,
                    d.identity,
                    d.question,
                    d.answer,
                    d.reason,
                    outcome_name(d.outcome),
                    tools_json(&d.tool_calls),
                    d.latency_ms,
                    d.input_tokens,
                    d.output_tokens,
                    d.cached_tokens,
                    d.context_tokens,
                    d.steps,
                    at,
                ],
            )
            .map(drop)
        })
    }

    fn delete_answer(&self, id: &ma::AnswerId) -> Result<(), String> {
        self.with(|db| {
            db.execute("DELETE FROM answers WHERE answer_id = ?1", [&id.0])
                .map(drop)
        })
    }

    fn vote(&self, id: &ma::VoteId) -> Result<Option<ma::VoteSnapshot>, String> {
        let row = self.with(|db| {
            db.query_row(&format!("{VOTE} WHERE vote_id = ?1"), [&id.0], vote_row)
                .optional()
        })?;
        row.transpose()
    }

    fn put_vote(&self, vote: &ma::VoteSnapshot) -> Result<(), String> {
        let d = &vote.data;
        let at = ms_of(&d.at)?;
        // The answer's presence is checked in the statement that writes the vote, so a purge
        // between the command's check and this write leaves no vote behind.
        let written = self.with(|db| {
            db.execute(
                "INSERT INTO votes (vote_id, answer_id, identity, value, comment, at_ms) \
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6 \
                 WHERE EXISTS (SELECT 1 FROM answers WHERE answer_id = ?2) \
                 ON CONFLICT (vote_id) DO UPDATE SET answer_id = ?2, identity = ?3, value = ?4, \
                 comment = ?5, at_ms = ?6",
                params![
                    d.vote_id.0,
                    d.answer_id.0,
                    d.identity,
                    value_name(d.value),
                    d.comment,
                    at
                ],
            )
        })?;
        if written == 0 {
            return Err(NO_SUCH_ANSWER.into());
        }
        Ok(())
    }

    fn delete_vote(&self, id: &ma::VoteId) -> Result<(), String> {
        self.with(|db| {
            db.execute("DELETE FROM votes WHERE vote_id = ?1", [&id.0])
                .map(drop)
        })
    }

    fn review(&self, id: &ma::ReviewId) -> Result<Option<ma::ReviewItemSnapshot>, String> {
        let row = self.with(|db| {
            db.query_row(
                &format!("{REVIEW} WHERE review_id = ?1"),
                [&id.0],
                review_row,
            )
            .optional()
        })?;
        row.transpose()
    }

    fn put_review(&self, item: &ma::ReviewItemSnapshot) -> Result<(), String> {
        let d = &item.data;
        let opened = ms_of(&d.opened_at)?;
        let closed = d.closed_at.as_ref().map(ms_of).transpose()?;
        // Each write holds its condition in its own statement: a close between another close's
        // read and its write, or a purge between an open's check and its write, changes no row.
        match item.state {
            ma::ReviewItemState::Open => {
                let (written, kept) = self.with(|db| {
                    let written = db.execute(
                        "INSERT INTO reviews (review_id, answer_id, cause, state, opened_ms, \
                         close_reason, closed_ms) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7 \
                         WHERE EXISTS (SELECT 1 FROM answers WHERE answer_id = ?2) \
                         ON CONFLICT (review_id) DO NOTHING",
                        params![
                            d.review_id.0,
                            d.answer_id.0,
                            cause_name(d.cause),
                            review_state_name(item.state),
                            opened,
                            d.close_reason,
                            closed
                        ],
                    )?;
                    let kept = db
                        .query_row(
                            "SELECT 1 FROM answers WHERE answer_id = ?1",
                            [&d.answer_id.0],
                            |_| Ok(()),
                        )
                        .optional()?
                        .is_some();
                    Ok((written, kept))
                })?;
                // An item opened at once by another request is the one this open meant.
                if written == 0 && !kept {
                    return Err(NO_SUCH_ANSWER.into());
                }
                Ok(())
            }
            ma::ReviewItemState::Closed => {
                let written = self.with(|db| {
                    db.execute(
                        "UPDATE reviews SET state = ?2, close_reason = ?3, closed_ms = ?4 \
                         WHERE review_id = ?1 AND state = ?5",
                        params![
                            d.review_id.0,
                            review_state_name(item.state),
                            d.close_reason,
                            closed,
                            review_state_name(ma::ReviewItemState::Open)
                        ],
                    )
                })?;
                if written == 0 {
                    return Err(REVIEW_NOT_OPEN.into());
                }
                Ok(())
            }
        }
    }

    fn delete_review(&self, id: &ma::ReviewId) -> Result<(), String> {
        self.with(|db| {
            db.execute("DELETE FROM reviews WHERE review_id = ?1", [&id.0])
                .map(drop)
        })
    }

    fn answers(&self) -> Result<Vec<ma::AnswerRecordData>, String> {
        Records::answers(self)
    }

    fn votes(&self, answer_id: &str) -> Result<Vec<ma::VoteData>, String> {
        Records::votes(self, answer_id)
    }

    fn reviews(&self, answer_id: &str) -> Result<Vec<ma::ReviewItemSnapshot>, String> {
        Records::reviews(self, answer_id)
    }

    fn open_reviews(&self) -> Result<Vec<ma::OpenReviews>, String> {
        let rows = self.with(|db| {
            let mut q = db.prepare(&format!(
                "{REVIEW} WHERE state = 'Open' ORDER BY opened_ms, rowid"
            ))?;
            let rows = q.query_map([], review_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        rows.into_iter()
            .map(|r| {
                r.map(|item| ma::OpenReviews {
                    review_id: item.data.review_id,
                    answer_id: item.data.answer_id,
                    cause: item.data.cause,
                    opened_at: item.data.opened_at,
                })
            })
            .collect()
    }

    fn purge(&self, before_ms: i64) -> Result<Purge, String> {
        self.with(|db| {
            let tx = db.transaction()?;
            // A vote or an item whose answer is gone has no meaning: it goes too, whatever left it.
            let expired = "answer_id IN (SELECT answer_id FROM answers WHERE at_ms < ?1) \
                OR answer_id NOT IN (SELECT answer_id FROM answers)";
            let votes = tx.execute(&format!("DELETE FROM votes WHERE {expired}"), [before_ms])?;
            let reviews =
                tx.execute(&format!("DELETE FROM reviews WHERE {expired}"), [before_ms])?;
            let records = tx.execute("DELETE FROM answers WHERE at_ms < ?1", [before_ms])?;
            tx.commit()?;
            Ok(Purge {
                records: records as i64,
                votes: votes as i64,
                reviews: reviews as i64,
            })
        })
    }
}

pub fn outcome_name(o: ma::AnswerOutcome) -> &'static str {
    match o {
        ma::AnswerOutcome::Answered => "answered",
        ma::AnswerOutcome::Failed => "failed",
        ma::AnswerOutcome::Limited => "limited",
    }
}

fn outcome_of(name: &str) -> Option<ma::AnswerOutcome> {
    [
        ma::AnswerOutcome::Answered,
        ma::AnswerOutcome::Failed,
        ma::AnswerOutcome::Limited,
    ]
    .into_iter()
    .find(|o| outcome_name(*o) == name)
}

pub fn value_name(v: ma::VoteValue) -> &'static str {
    match v {
        ma::VoteValue::Up => "up",
        ma::VoteValue::Down => "down",
    }
}

pub fn value_of(name: &str) -> Option<ma::VoteValue> {
    [ma::VoteValue::Up, ma::VoteValue::Down]
        .into_iter()
        .find(|v| value_name(*v) == name)
}

pub fn cause_name(c: ma::ReviewCause) -> &'static str {
    match c {
        ma::ReviewCause::VotedDown => "voted-down",
        ma::ReviewCause::Failed => "failed",
        ma::ReviewCause::Slow => "slow",
    }
}

pub fn cause_of(name: &str) -> Option<ma::ReviewCause> {
    [
        ma::ReviewCause::VotedDown,
        ma::ReviewCause::Failed,
        ma::ReviewCause::Slow,
    ]
    .into_iter()
    .find(|c| cause_name(*c) == name)
}

pub fn review_state_name(s: ma::ReviewItemState) -> &'static str {
    match s {
        ma::ReviewItemState::Open => "Open",
        ma::ReviewItemState::Closed => "Closed",
    }
}

fn review_state_of(name: &str) -> Option<ma::ReviewItemState> {
    [ma::ReviewItemState::Open, ma::ReviewItemState::Closed]
        .into_iter()
        .find(|s| review_state_name(*s) == name)
}

/// The `tool` events of an answer as JSON, as the stream carries them.
pub fn tools_json(tools: &[ma::AskTool]) -> String {
    Value::Array(
        tools
            .iter()
            .map(|t| {
                json!({
                    "step": t.step,
                    "name": t.name,
                    "arguments": t.arguments,
                    "result_bytes": t.result_bytes,
                    "truncated": t.truncated,
                    "failed": t.failed,
                    "withheld": t.withheld,
                })
            })
            .collect(),
    )
    .to_string()
}

fn tools_of(text: &str) -> Option<Vec<ma::AskTool>> {
    let tools: Vec<Value> = serde_json::from_str(text).ok()?;
    tools
        .iter()
        .map(|t| {
            Some(ma::AskTool {
                step: t.get("step")?.as_i64()?,
                name: t.get("name")?.as_str()?.to_string(),
                arguments: t.get("arguments")?.as_str()?.to_string(),
                result_bytes: t.get("result_bytes")?.as_i64()?,
                truncated: t.get("truncated")?.as_bool()?,
                failed: t.get("failed")?.as_bool()?,
                // Kept since redaction; a record from before it was never withheld.
                withheld: t.get("withheld").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

/// An instant as a `Timestamp`, RFC 3339 in UTC to the second.
pub fn stamp(ms: i64) -> Timestamp {
    Timestamp(crate::sources::rfc3339(ms))
}

/// The milliseconds since the Unix epoch of an RFC 3339 instant.
pub fn ms_of(t: &Timestamp) -> Result<i64, String> {
    parse_rfc3339(&t.0).ok_or_else(|| format!("{:?} is not an RFC 3339 instant", t.0))
}

/// `YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM)` as milliseconds since the Unix epoch.
fn parse_rfc3339(t: &str) -> Option<i64> {
    let b = t.as_bytes();
    let digits = |from: usize, to: usize| -> Option<i64> {
        let part = t.get(from..to)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't' | b' ')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let (y, mo, d) = (digits(0, 4)?, digits(5, 7)?, digits(8, 10)?);
    let (h, mi, s) = (digits(11, 13)?, digits(14, 16)?, digits(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let mut rest = &t[19..];
    let mut ms = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let n = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return None;
        }
        let first: String = fraction[..n].chars().chain("000".chars()).take(3).collect();
        ms = first.parse::<i64>().ok()?;
        rest = &fraction[n..];
    }
    let offset_s = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let o = rest.as_bytes();
            if o.len() != 6 || o[3] != b':' {
                return None;
            }
            let oh: i64 = rest[1..3].parse().ok()?;
            let om: i64 = rest[4..6].parse().ok()?;
            sign * (oh * 3600 + om * 60)
        }
    };
    // Howard Hinnant's `days_from_civil`.
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + h * 3600 + mi * 60 + s) - offset_s) * 1000 + ms)
}

/// What the service knows of one question once its stream ended, for [`Keeper::record`].
pub struct Asked {
    pub answer_id: String,
    pub identity: String,
    pub question: String,
    /// The `tool` events of the answer, in order.
    pub tool_calls: Vec<ma::AskTool>,
    pub latency_ms: i64,
    /// When the question was asked, in Unix milliseconds.
    pub at_ms: i64,
}

/// Why a vote was not cast.
#[derive(Debug)]
pub enum VoteRefused {
    NoSuchAnswer,
    Store(String),
}

/// The records of one instance, as the service keeps them.
#[derive(Clone)]
pub struct Keeper {
    pub store: Arc<dyn RecordStore>,
    pub instance: String,
    pub retention_days: i64,
    pub slow_after_ms: Option<i64>,
}

impl Keeper {
    /// The records of the instance `spec` names, in `home`: the schema `ask.records` names,
    /// through the records process its connection launches ([`postgres::PgRecords`]), or
    /// `ask.sqlite` in its instance directory when there is no `ask.records` (`ask::check` holds
    /// `ask.records` to a `postgres` store).
    pub fn open(home: &Home, spec: &m::InstanceSpec) -> Result<Self, String> {
        let ask = spec.ask.as_ref();
        let store: Arc<dyn RecordStore> = match ask.and_then(|a| a.records.as_ref()) {
            Some(records) => Arc::new(postgres::PgRecords::launched(postgres::launch_of(
                spec, records,
            )?)?),
            None => Arc::new(Records::open(&home.instance_dir(&spec.name.0).join(FILE))?),
        };
        Ok(Self {
            store,
            instance: spec.name.0.clone(),
            retention_days: ask
                .and_then(|a| a.retention_days)
                .unwrap_or(DEFAULT_RETENTION_DAYS),
            slow_after_ms: ask.and_then(|a| a.slow_after_ms),
        })
    }

    /// Runs `command` over the generated behaviours on the store, with the `external:` branches
    /// `decided` sets; a failure of the store during it is the `Err`.
    fn run<T>(
        &self,
        decided: impl FnOnce(&mut Shared),
        command: impl FnOnce(&mut Generated<AskPorts>) -> Result<T, UnmetObligation>,
    ) -> Result<T, String> {
        let mut shared = Shared::default();
        decided(&mut shared);
        let mut app = Generated::new(AskPorts::new(
            self.store.clone(),
            Rc::new(RefCell::new(shared)),
        ));
        let result = command(&mut app);
        if let Some(fault) = app.ports.take_fault() {
            return Err(fault);
        }
        result.map_err(|e| format!("{}: {}", e.capability, e.source))
    }

    /// Records one answer (`cortex.ask.RecordAnswer`), then opens its review items: `failed` for
    /// an outcome other than `answered`, `slow` for a latency past `slow_after_ms`. The question,
    /// the answer and the tools' arguments are kept with their credential shapes masked.
    pub fn record(&self, asked: Asked, outcome: &ma::AskOutcome) -> Result<(), String> {
        let zero = ma::AskDone {
            answer_id: String::new(),
            answer: String::new(),
            nodes: vec![],
            evidence: vec![],
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            context_tokens: 0,
            steps: 0,
            unrestored: 0,
        };
        let (kind, done, reason) = match outcome {
            ma::AskOutcome::Answered { answered } => {
                (ma::AnswerOutcome::Answered, &answered.done, None)
            }
            ma::AskOutcome::LimitReached { error } => {
                (ma::AnswerOutcome::Limited, &zero, Some(error.limit.clone()))
            }
            ma::AskOutcome::ModelFailed { .. } => (
                ma::AnswerOutcome::Failed,
                &zero,
                Some("model-failed".into()),
            ),
            ma::AskOutcome::ReaderFailed { .. } => (
                ma::AnswerOutcome::Failed,
                &zero,
                Some("reader-failed".into()),
            ),
            // A refusal before the answer starts is no answer and leaves no record.
            ma::AskOutcome::Unidentified { .. } | ma::AskOutcome::RateLimited { .. } => {
                return Ok(())
            }
        };
        let input = ma::RecordAnswer {
            answer_id: ma::AnswerId(asked.answer_id.clone()),
            instance_name: m::InstanceName(self.instance.clone()),
            identity: asked.identity,
            question: masked(&asked.question),
            answer: (kind == ma::AnswerOutcome::Answered).then(|| masked(&done.answer)),
            reason,
            outcome: kind,
            // A tool's arguments are the model's, and can repeat what the question carried.
            tool_calls: asked
                .tool_calls
                .into_iter()
                .map(|t| ma::AskTool {
                    arguments: masked(&t.arguments),
                    ..t
                })
                .collect(),
            latency_ms: asked.latency_ms.max(0),
            input_tokens: done.input_tokens,
            output_tokens: done.output_tokens,
            cached_tokens: done.cached_tokens,
            context_tokens: done.context_tokens,
            steps: done.steps,
            at: stamp(asked.at_ms),
        };
        self.run(|_| {}, |app| app.record_answer(input))?;
        if kind != ma::AnswerOutcome::Answered {
            self.open_review(&asked.answer_id, ma::ReviewCause::Failed)?;
        }
        if self
            .slow_after_ms
            .is_some_and(|slow| asked.latency_ms > slow)
        {
            self.open_review(&asked.answer_id, ma::ReviewCause::Slow)?;
        }
        Ok(())
    }

    /// Opens the review item of `answer_id` for `cause` (`cortex.ask.OpenReview`); an item the
    /// cause already opened, open or closed, stays as it is.
    fn open_review(&self, answer_id: &str, cause: ma::ReviewCause) -> Result<(), String> {
        let input = ma::OpenReview {
            review_id: ma::ReviewId(format!("{answer_id}:{}", cause_name(cause))),
            answer_id: ma::AnswerId(answer_id.into()),
            cause,
        };
        self.run(|_| {}, |app| app.open_review(input)).map(drop)
    }

    /// `identity`'s vote on `answer_id` (`cortex.ask.CastVote`), cast or changed; a `down` vote
    /// opens the answer's `voted-down` review item. The vote as it is kept, its comment with its
    /// credential shapes masked. An answer a purge takes during the vote refuses it as one that
    /// was never kept; the purge takes the vote with the answer.
    pub fn vote(
        &self,
        answer_id: &str,
        identity: &str,
        value: ma::VoteValue,
        comment: Option<String>,
    ) -> Result<ma::VoteData, VoteRefused> {
        let id = ma::AnswerId(answer_id.into());
        let missing = self
            .store
            .answer(&id)
            .map_err(VoteRefused::Store)?
            .is_none();
        let vote_id = ma::VoteId(format!("{answer_id}:{identity}"));
        let input = ma::CastVote {
            vote_id: vote_id.clone(),
            answer_id: id,
            identity: identity.into(),
            value,
            comment: comment.as_deref().map(masked),
        };
        let outcome = self
            .run(
                |shared| {
                    shared
                        .external
                        .insert((CAST_VOTE, "no-such-answer"), missing);
                },
                |app| app.cast_vote(input),
            )
            .map_err(refused)?;
        match outcome {
            ma::CastVoteOutcome::NoSuchAnswer { .. } => return Err(VoteRefused::NoSuchAnswer),
            ma::CastVoteOutcome::Unidentified { .. } => {
                return Err(VoteRefused::Store(
                    "an unidentified vote reached the store".into(),
                ))
            }
            ma::CastVoteOutcome::Cast { .. } | ma::CastVoteOutcome::Changed { .. } => {}
        }
        if value == ma::VoteValue::Down {
            self.open_review(answer_id, ma::ReviewCause::VotedDown)
                .map_err(refused)?;
        }
        // Only a purge deletes a vote, and it takes the answer with it.
        match self.store.vote(&vote_id).map_err(VoteRefused::Store)? {
            Some(vote) => Ok(vote.data),
            None => Err(VoteRefused::NoSuchAnswer),
        }
    }

    /// The open review items (`cortex.ask.OpenReviews`), oldest first.
    pub fn open_reviews(&self) -> Result<Vec<ma::OpenReviews>, String> {
        self.run(|_| {}, |app| app.open_reviews())
    }

    /// Closes a review item with `reason` (`cortex.ask.CloseReview`): the outcome, and the item as
    /// it is kept after it. The reason is kept with its credential shapes masked. Of two closes of
    /// one item at once, the one whose write finds the item closed answers `wrong-state`, and one
    /// whose item a purge took answers `no-such-review`.
    pub fn close(
        &self,
        review_id: &str,
        reason: &str,
    ) -> Result<(ma::CloseReviewOutcome, Option<ma::ReviewItemSnapshot>), String> {
        let id = ma::ReviewId(review_id.into());
        let input = ma::CloseReview {
            review_id: id.clone(),
            reason: masked(reason),
        };
        let outcome = match self.run(|_| {}, |app| app.close_review(input)) {
            Err(e) if e == REVIEW_NOT_OPEN => None,
            other => Some(other?),
        };
        let item = self.store.review(&id)?;
        let outcome = match (outcome, &item) {
            (Some(outcome), _) => outcome,
            (None, Some(_)) => ma::CloseReviewOutcome::WrongState {
                error: ma::ReviewNotOpen { review_id: id },
            },
            (None, None) => ma::CloseReviewOutcome::NoSuchReview {
                error: ma::ReviewNotFound { review_id: id },
            },
        };
        Ok((outcome, item))
    }

    /// Purges every record older than `retention_days` before `now_ms`, with its votes and review
    /// items (`cortex.ask.PurgeExpired`). A retention too long to count before now keeps every
    /// record: the cutoff stops at [`EARLIEST_MS`] rather than wrap.
    pub fn purge(&self, now_ms: i64) -> Result<ma::Purged, String> {
        let before_ms = now_ms
            .saturating_sub(self.retention_days.saturating_mul(DAY_MS))
            .max(EARLIEST_MS);
        let purged = self.store.purge(before_ms)?;
        let outcome = self.run(
            |shared| {
                shared
                    .integers
                    .extend([purged.records, purged.votes, purged.reviews]);
            },
            |app| {
                app.purge_expired(ma::PurgeExpired {
                    before: stamp(before_ms),
                })
            },
        )?;
        let ma::PurgeExpiredOutcome::Purged { purged } = outcome;
        Ok(purged)
    }
}

const CAST_VOTE: &str = "cortex.ask.CastVote";

const DAY_MS: i64 = 86_400_000;

/// The earliest instant a [`Timestamp`] writes, 0001-01-01T00:00:00Z, in Unix milliseconds: the
/// furthest back a purge's cutoff goes.
pub const EARLIEST_MS: i64 = -62_135_596_800_000;

/// `text` with every credential shape masked (`crate::mask`), as a record keeps it.
fn masked(text: &str) -> String {
    crate::mask::mask(text).0
}

/// A store failure during a vote as the vote's refusal: [`NO_SUCH_ANSWER`] is an answer a purge
/// took after the vote read it.
fn refused(e: String) -> VoteRefused {
    if e == NO_SUCH_ANSWER {
        VoteRefused::NoSuchAnswer
    } else {
        VoteRefused::Store(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instant_reads_back_as_it_was_written() {
        for ms in [0, 1_600_000_000_000, 1_792_000_000_000, -86_400_000] {
            let at = stamp(ms);
            assert_eq!(ms_of(&at).unwrap(), ms, "{}", at.0);
        }
        assert_eq!(
            parse_rfc3339("2020-01-01T01:00:00.250+01:00"),
            Some(1_577_836_800_250)
        );
        assert_eq!(
            parse_rfc3339("2020-01-01 00:00:00Z"),
            Some(1_577_836_800_000)
        );
        assert_eq!(parse_rfc3339("2020-13-01T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("yesterday"), None);
    }

    #[test]
    fn a_purge_takes_the_old_records_with_their_votes_and_reviews() {
        let store = Arc::new(Records::in_memory().unwrap());
        let keeper = Keeper {
            store: store.clone(),
            instance: "i".into(),
            retention_days: 1,
            slow_after_ms: None,
        };
        let now = 10 * 86_400_000;
        for (id, at_ms) in [("old", now - 2 * 86_400_000), ("new", now)] {
            let asked = Asked {
                answer_id: id.into(),
                identity: "a".into(),
                question: "q".into(),
                tool_calls: vec![],
                latency_ms: 1,
                at_ms,
            };
            let failed = ma::AskOutcome::ModelFailed {
                error: ma::ModelFailed { reason: "x".into() },
            };
            keeper.record(asked, &failed).unwrap();
            keeper.vote(id, "b", ma::VoteValue::Down, None).unwrap();
        }
        let purged = keeper.purge(now).unwrap();
        assert_eq!((purged.records, purged.votes, purged.reviews), (1, 1, 2));
        let left: Vec<String> = store
            .answers()
            .unwrap()
            .into_iter()
            .map(|a| a.answer_id.0)
            .collect();
        assert_eq!(left, ["new"]);
        assert_eq!(store.reviews("new").unwrap().len(), 2);
    }

    #[test]
    fn a_file_from_a_newer_cortex_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        drop(Records::open(&path).unwrap());
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(db);
        let refused = Records::open(&path).err().expect("refused");
        assert!(refused.contains("newer"), "{refused}");
        // An open at the current version changes nothing.
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", SCHEMA_VERSION)
            .unwrap();
        drop(db);
        assert!(Records::open(&path).is_ok());
    }
}

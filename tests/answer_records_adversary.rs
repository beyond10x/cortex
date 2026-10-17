//! Adversary cases for `story:answer-records-and-review`: the answer records behind
//! `cortex_cli::ask::records::Keeper`, driven without the HTTP side.
//!
//! Two of them interleave a second request between the read and the write of a command. The
//! service runs every record call on the blocking pool of a multi-threaded runtime, and the
//! SQLite backend holds its lock for one statement at a time, so a second request can land
//! there; the cases place it there deterministically, through a `RecordStore` that runs a hook
//! before its first write of one kind.

use std::sync::{Arc, Mutex};

use cortex_cli::ask::records::{Asked, Keeper, Purge, RecordStore, Records};
use cortex_model::ask as ma;

const DAY: i64 = 86_400_000;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn keeper(store: Arc<dyn RecordStore>, retention_days: i64) -> Keeper {
    Keeper {
        store,
        instance: "asker".into(),
        retention_days,
        slow_after_ms: None,
    }
}

fn asked(answer_id: &str, question: &str, at_ms: i64) -> Asked {
    Asked {
        answer_id: answer_id.into(),
        identity: "alice".into(),
        question: question.into(),
        tool_calls: vec![],
        latency_ms: 1,
        at_ms,
    }
}

fn failed() -> ma::AskOutcome {
    ma::AskOutcome::ModelFailed {
        error: ma::ModelFailed {
            reason: "the gateway was down".into(),
        },
    }
}

type Hook = Box<dyn FnOnce() + Send>;

/// The records of `inner`, with a hook run once, before the first vote or review item is written.
struct Interleaved {
    inner: Arc<Records>,
    before_vote: Mutex<Option<Hook>>,
    before_review: Mutex<Option<Hook>>,
}

impl Interleaved {
    fn new(inner: Arc<Records>) -> Self {
        Self {
            inner,
            before_vote: Mutex::new(None),
            before_review: Mutex::new(None),
        }
    }

    fn fire(slot: &Mutex<Option<Hook>>) {
        let hook = slot.lock().unwrap().take();
        if let Some(hook) = hook {
            hook();
        }
    }
}

impl RecordStore for Interleaved {
    fn answer(&self, id: &ma::AnswerId) -> Result<Option<ma::AnswerRecordSnapshot>, String> {
        self.inner.answer(id)
    }
    fn put_answer(&self, record: &ma::AnswerRecordSnapshot) -> Result<(), String> {
        self.inner.put_answer(record)
    }
    fn delete_answer(&self, id: &ma::AnswerId) -> Result<(), String> {
        self.inner.delete_answer(id)
    }
    fn vote(&self, id: &ma::VoteId) -> Result<Option<ma::VoteSnapshot>, String> {
        self.inner.vote(id)
    }
    fn put_vote(&self, vote: &ma::VoteSnapshot) -> Result<(), String> {
        Self::fire(&self.before_vote);
        self.inner.put_vote(vote)
    }
    fn delete_vote(&self, id: &ma::VoteId) -> Result<(), String> {
        self.inner.delete_vote(id)
    }
    fn review(&self, id: &ma::ReviewId) -> Result<Option<ma::ReviewItemSnapshot>, String> {
        self.inner.review(id)
    }
    fn put_review(&self, item: &ma::ReviewItemSnapshot) -> Result<(), String> {
        Self::fire(&self.before_review);
        self.inner.put_review(item)
    }
    fn delete_review(&self, id: &ma::ReviewId) -> Result<(), String> {
        self.inner.delete_review(id)
    }
    fn answers(&self) -> Result<Vec<ma::AnswerRecordData>, String> {
        RecordStore::answers(&*self.inner)
    }
    fn votes(&self, answer_id: &str) -> Result<Vec<ma::VoteData>, String> {
        RecordStore::votes(&*self.inner, answer_id)
    }
    fn reviews(&self, answer_id: &str) -> Result<Vec<ma::ReviewItemSnapshot>, String> {
        RecordStore::reviews(&*self.inner, answer_id)
    }
    fn open_reviews(&self) -> Result<Vec<ma::OpenReviews>, String> {
        self.inner.open_reviews()
    }
    fn purge(&self, before_ms: i64) -> Result<Purge, String> {
        self.inner.purge(before_ms)
    }
}

/// Two reviewers close one open item at once: one close wins, the other answers `wrong-state`
/// (409) and the item keeps the first reason (`cortex.ask.ReviewNotOpen`). The second close lands
/// between the first one's read of the item and its write.
#[test]
fn two_closes_of_one_review_item_at_once_close_it_once() {
    let inner = Arc::new(Records::in_memory().unwrap());
    let direct = keeper(inner.clone(), 90);
    direct
        .record(asked("a", "q?", now_ms()), &failed())
        .unwrap();

    let second = Arc::new(Mutex::new(None));
    let store = Interleaved::new(inner.clone());
    {
        let (direct, second) = (direct.clone(), second.clone());
        *store.before_review.lock().unwrap() = Some(Box::new(move || {
            let (outcome, _) = direct.close("a:failed", "second", vec![], vec![]).unwrap();
            *second.lock().unwrap() = Some(outcome);
        }));
    }
    let (first, _) = keeper(Arc::new(store), 90)
        .close("a:failed", "first", vec![], vec![])
        .unwrap();
    let second = second.lock().unwrap().take().expect("the second close ran");

    let closes = [&first, &second]
        .into_iter()
        .filter(|o| matches!(o, ma::CloseReviewOutcome::Closed { .. }))
        .count();
    assert_eq!(
        closes, 1,
        "both closes answered `closed`: first {first:?}, second {second:?}"
    );
}

/// A vote cast while the purge takes its answer leaves nothing behind: no vote and no review item
/// of an answer past retention outlives the purge (`cortex.ask.PurgeExpired`: "Nothing older than
/// `before` is left"; a vote and a review item "have no meaning without their answer").
#[test]
fn a_vote_cast_while_its_answer_is_purged_leaves_no_vote_or_review_item_behind() {
    let inner = Arc::new(Records::in_memory().unwrap());
    let now = now_ms();
    let direct = keeper(inner.clone(), 1);
    direct
        .record(asked("old", "q?", now - 2 * DAY), &failed())
        .unwrap();

    let store = Interleaved::new(inner.clone());
    {
        let direct = direct.clone();
        *store.before_vote.lock().unwrap() = Some(Box::new(move || {
            direct.purge(now).unwrap();
        }));
    }
    let _ = keeper(Arc::new(store), 1).vote(
        "old",
        "bob",
        ma::VoteValue::Down,
        Some("my comment".into()),
    );

    // The next purge, a day later, finds nothing to take either.
    direct.purge(now + DAY).unwrap();
    assert!(
        RecordStore::answer(&*inner, &ma::AnswerId("old".into()))
            .unwrap()
            .is_none(),
        "the answer was purged"
    );
    let votes = RecordStore::votes(&*inner, "old").unwrap();
    assert!(
        votes.is_empty(),
        "a vote outlives its purged answer, past retention: {votes:?}"
    );
    let open: Vec<String> = inner
        .open_reviews()
        .unwrap()
        .into_iter()
        .map(|r| r.review_id.0)
        .collect();
    assert!(
        open.is_empty(),
        "the review queue lists an item of a purged answer: {open:?}"
    );
}

/// A large `retention_days` keeps every record: `ask::check` refuses only 0 or less, and the spec
/// schema states no maximum, so `retention_days: 200000000000` (a spec author's "forever") is
/// served. The purge must not overflow into a cutoff in the future.
#[test]
fn a_retention_too_large_to_count_in_milliseconds_purges_nothing() {
    let store = Arc::new(Records::in_memory().unwrap());
    let now = now_ms();
    let k = keeper(store.clone(), 200_000_000_000);
    k.record(asked("today", "q?", now), &failed()).unwrap();
    let purged = k.purge(now).expect("the purge runs");
    assert_eq!(purged.records, 0, "{purged:?}");
    assert_eq!(RecordStore::answers(&*store).unwrap().len(), 1);
}

/// A credential pasted into a question is not kept as written: cortex masks credential shapes on
/// every run whatever the policy (`src/redact.rs`, module doc), and the `Credential` class is
/// irreversible "before the document is stored". `ask serve` hands the question to the record
/// unchanged (`serve::ask` -> `Asked.question`).
#[test]
fn a_credential_in_a_question_is_not_kept_in_the_answer_record() {
    let store = Arc::new(Records::in_memory().unwrap());
    // Assembled at run time, so no key-shaped literal sits in the source.
    let key = format!("{}{}", "AKIA", "Q3EGRT5YHVBNK7PZ");
    keeper(store.clone(), 90)
        .record(
            asked(
                "a",
                &format!("Is the access key {key} still valid?"),
                now_ms(),
            ),
            &failed(),
        )
        .unwrap();
    let records = RecordStore::answers(&*store).unwrap();
    assert_eq!(records.len(), 1);
    assert!(
        !records[0].question.contains(&key),
        "ask.sqlite keeps the credential as written: {:?}",
        records[0].question
    );
}

/// Ids shaped like SQL reach the store as values: a vote on one finds no answer, a close of one
/// finds no item, and the records are intact after.
#[test]
fn ids_shaped_like_sql_find_nothing_and_change_nothing() {
    let store = Arc::new(Records::in_memory().unwrap());
    let k = keeper(store.clone(), 90);
    k.record(asked("a", "q?", now_ms()), &failed()).unwrap();
    for id in [
        "x' OR '1'='1",
        "a'; DELETE FROM answers; --",
        "%",
        "a:failed' --",
    ] {
        assert!(matches!(
            k.vote(id, "bob", ma::VoteValue::Down, None),
            Err(cortex_cli::ask::records::VoteRefused::NoSuchAnswer)
        ));
        let (outcome, item) = k.close(id, "reason", vec![], vec![]).unwrap();
        assert!(
            matches!(outcome, ma::CloseReviewOutcome::NoSuchReview { .. }),
            "{id}: {outcome:?}"
        );
        assert!(item.is_none());
    }
    assert_eq!(RecordStore::answers(&*store).unwrap().len(), 1);
    assert_eq!(k.open_reviews().unwrap().len(), 1);
}

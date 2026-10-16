//! The spend of the day of one instance (`cortex.instance.DailySpend`): `spend.json` in the
//! instance directory, the sum of the costs its model answers stated and of the tokens they
//! reported on one UTC calendar day, and the daily ceilings (`model.daily_ceiling_usd`,
//! `model.daily_ceiling_tokens`) every model call of the instance is held to.

use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::{json, Value};

use crate::extract::Usage;
use crate::home::{write_atomic, Home};
use crate::instance::{now_ms, Layout};

/// The `format` of `spend.json`.
pub const FORMAT: &str = "cortex.daily-spend/1";

/// `spend.json` in the instance directory.
pub fn path(layout: &Layout) -> PathBuf {
    layout.dir.join("spend.json")
}

/// The UTC calendar day of `ms`, Unix milliseconds, as `YYYY-MM-DD`.
pub fn day(ms: i64) -> String {
    let stamp = crate::quality::utc_stamp(ms);
    format!("{}-{}-{}", &stamp[0..4], &stamp[4..6], &stamp[6..8])
}

/// The tokens one answer's `usage` reported: its input, cache reads, cache writes and output
/// together. `None` when its tokens cannot be counted: the answer did not report each of them,
/// reported one below 0, or their sum does not fit an `i64`.
pub fn tokens(usage: &Usage) -> Option<i64> {
    [
        usage.input_tokens,
        usage.cache_read_input_tokens,
        usage.cache_creation_input_tokens,
        usage.output_tokens,
    ]
    .into_iter()
    .try_fold(0i64, |sum, n| sum.checked_add(n.filter(|n| *n >= 0)?))
}

/// Whether the cost `claude` reports for an answer of `model` counts against its dollar limits:
/// not behind a model gateway (`model.gateway`), where it is priced at `claude`'s default model's
/// rate rather than observed, and the instance is bounded in tokens only. An instance created with
/// a gateway before `model.budget_tokens` was required for one (`spec::gateway_refusal`) has no
/// token budget in its frozen spec: its dollar limits count as before, so it is never unbounded.
pub fn counts_dollars(model: &m::ModelSpec) -> bool {
    model.gateway.is_none() || model.budget_tokens.is_none()
}

/// The dollar budget of one source run, quality measurement or schema proposal
/// (`model.budget_usd`): `None` behind a model gateway ([`counts_dollars`]), where no dollar limit
/// bounds anything. A spec without a gateway always names one (`spec::check`).
pub fn dollar_budget(model: &m::ModelSpec) -> Option<f64> {
    if !counts_dollars(model) {
        return None;
    }
    Some(
        model
            .budget_usd
            .as_ref()
            .map_or(0.0, |b| b.0.parse().unwrap_or(0.0)),
    )
}

/// The why of an answer whose tokens could not be counted ([`tokens`]), after the limit's name.
const UNCOUNTED: &str = "the tokens of an answer could not be counted: its usage reported no \
                         count, a count below 0, or more than can be summed";

/// A run's budget in tokens (`model.budget_tokens`), for one source run, quality measurement or
/// schema proposal, and the tokens its answers reported.
pub struct TokenBudget {
    cap: Option<i64>,
    /// `None` from the first answer whose tokens could not be counted, or when the sum would not
    /// fit an `i64`.
    used: Option<i64>,
}

impl TokenBudget {
    /// No tokens used yet of `model.budget_tokens`.
    pub fn new(model: &m::ModelSpec) -> Self {
        Self {
            cap: model.budget_tokens,
            used: Some(0),
        }
    }

    /// Adds one answer's tokens ([`tokens`]), a failed answer's included.
    pub fn add(&mut self, usage: &Usage) {
        self.used = self.used.and_then(|used| used.checked_add(tokens(usage)?));
    }

    /// Whether a further model call is allowed: an error naming the budget once the tokens used
    /// reach it, or, under a budget, once an answer's tokens could not be counted.
    pub fn check(&self) -> Result<(), String> {
        match (self.cap, self.used) {
            (Some(cap), None) => Err(format!("token budget of {cap} tokens: {UNCOUNTED}")),
            (Some(cap), Some(used)) if used >= cap => Err(format!(
                "token budget of {cap} tokens spent: {used} tokens used"
            )),
            _ => Ok(()),
        }
    }
}

/// Why [`Daily::check`] allows no further model call.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    /// The reason, naming the daily ceiling.
    pub reason: String,
    /// Whether the day's spend or tokens reached a ceiling; `false` when the call is refused for
    /// another reason (an answer's tokens could not be counted, the spend of the day could not be
    /// read or recorded).
    pub reached: bool,
}

impl Refusal {
    fn not_reached(reason: String) -> Self {
        Self {
            reason,
            reached: false,
        }
    }
}

/// Micro-dollars in one US dollar. The spend of the day is summed in whole micro-dollars, so a sum
/// of decimal costs reaches a ceiling it equals (0.7 + 0.1 USD is 0.8 USD, not 0.7999…).
const MICROS: f64 = 1_000_000.0;

/// `usd` in whole micro-dollars, rounded to the nearest.
fn micros(usd: f64) -> i64 {
    (usd * MICROS).round() as i64
}

/// What `spend.json` says was spent on `day`: `0` when there is no file or it counts another day.
pub fn spent_on(layout: &Layout, day: &str) -> Result<f64, String> {
    spent(layout, day).map(|s| s.micros as f64 / MICROS)
}

/// The tokens `spend.json` says the answers of `day` reported: `0` when there is no file, it
/// counts another day, or it was written before tokens were counted (no `spent_tokens`).
pub fn tokens_on(layout: &Layout, day: &str) -> Result<i64, String> {
    spent(layout, day).map(|s| s.tokens)
}

/// What `spend.json` holds for one day.
#[derive(Default)]
struct Spent {
    /// `spent_usd` in whole micro-dollars.
    micros: i64,
    /// `spent_tokens`, `0` when absent.
    tokens: i64,
    /// Whether the file of that day carried `spent_tokens`.
    counts_tokens: bool,
    /// `uncounted_usd` in whole micro-dollars, `None` when absent: the costs answers reported
    /// behind a model gateway, which count against no dollar limit.
    uncounted: Option<i64>,
}

/// A decimal string field of `spend.json` in whole micro-dollars: an error unless it is a finite
/// number of 0 or more.
fn usd_field(value: &Value, field: &str, path: &std::path::Path) -> Result<i64, String> {
    value[field]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(micros)
        .ok_or_else(|| format!("{}: {field} is not a number of 0 or more", path.display()))
}

/// [`spent_on`] and [`tokens_on`] together. A `spent_usd` of today that is not a finite number of
/// 0 or more, or a `spent_tokens` that is not a whole number of 0 or more, is an error, so a
/// ceiling it would lift fails closed; so is an `uncounted_usd` that is not a finite number of 0
/// or more.
fn spent(layout: &Layout, day: &str) -> Result<Spent, String> {
    let path = path(layout);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Spent::default()),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{} is not JSON: {e}", path.display()))?;
    if value["format"] != FORMAT {
        return Err(format!("{} is not a {FORMAT} document", path.display()));
    }
    if value["day"].as_str() != Some(day) {
        return Ok(Spent::default());
    }
    let micros = usd_field(&value, "spent_usd", &path)?;
    let uncounted = match value.get("uncounted_usd") {
        None => None,
        Some(_) => Some(usd_field(&value, "uncounted_usd", &path)?),
    };
    let (tokens, counts_tokens) = match value.get("spent_tokens") {
        None => (0, false),
        Some(n) => (
            n.as_i64().filter(|n| *n >= 0).ok_or_else(|| {
                format!(
                    "{}: spent_tokens is not a whole number of 0 or more",
                    path.display()
                )
            })?,
            true,
        ),
    };
    Ok(Spent {
        micros,
        tokens,
        counts_tokens,
        uncounted,
    })
}

/// The daily ceilings of one command's model calls, and the record of what they spend.
pub struct Daily<'a> {
    layout: &'a Layout,
    /// The ceiling in whole micro-dollars, as the spend of the day is summed.
    ceiling: Option<i64>,
    /// The ceiling in tokens.
    token_ceiling: Option<i64>,
    /// The home whose lock [`Daily::add`] takes; `None` when the command holds it already.
    home: Option<&'a Home>,
    /// Why an answer's cost could not be recorded: with a ceiling, no further call is allowed.
    unrecorded: Option<String>,
    /// An answer's tokens could not be counted: with a ceiling in tokens, no further call is
    /// allowed.
    uncounted: bool,
    /// Whether an answer's cost adds to `spent_usd`; behind a model gateway it adds to
    /// `uncounted_usd` instead ([`counts_dollars`]).
    counts_dollars: bool,
}

impl<'a> Daily<'a> {
    /// The ceilings `model` names, for a command that holds the home's lock (`home` `None`) or
    /// takes it for each record (`home` the home). Behind a model gateway the ceiling in dollars
    /// bounds nothing ([`counts_dollars`]): only the ceiling in tokens is held.
    pub fn new(layout: &'a Layout, model: &m::ModelSpec, home: Option<&'a Home>) -> Self {
        let counts_dollars = counts_dollars(model);
        Self {
            layout,
            ceiling: model
                .daily_ceiling_usd
                .as_ref()
                .filter(|_| counts_dollars)
                .and_then(|c| c.0.parse::<f64>().ok())
                .map(micros),
            token_ceiling: model.daily_ceiling_tokens,
            home,
            unrecorded: None,
            uncounted: false,
            counts_dollars,
        }
    }

    /// The ceiling a refusal that concerns both names: the dollar one when there is one.
    fn named(&self) -> String {
        match (self.ceiling, self.token_ceiling) {
            (Some(cap), _) => format!("daily ceiling of {} USD", cap as f64 / MICROS),
            (None, Some(cap)) => format!("daily ceiling of {cap} tokens"),
            (None, None) => "daily ceiling".into(),
        }
    }

    /// What the next model call may spend: the lesser of `left`, what remains of the command's
    /// own budget, and what the ceiling in dollars leaves of today. An error, naming the daily
    /// ceiling, when either ceiling leaves nothing, what was spent today cannot be known, or,
    /// under the ceiling in tokens, an answer's tokens could not be counted.
    pub fn allow(&self, left: f64) -> Result<f64, String> {
        self.check(left).map_err(|refusal| refusal.reason)
    }

    /// [`Daily::check`] for a command's dollar bound `left` ([`dollar_budget`] less what it
    /// spent): `None` when no dollar budget applies (behind a model gateway, see
    /// [`counts_dollars`]), and then the call is bounded in dollars only by a ceiling, when there
    /// is one. The answer is the call's dollar bound, `None` when nothing bounds it in dollars.
    pub fn check_bound(&self, left: Option<f64>) -> Result<Option<f64>, Refusal> {
        match left {
            Some(left) => self.check(left).map(Some),
            None => {
                let allowed = self.check(f64::MAX)?;
                Ok(self.ceiling.is_some().then_some(allowed))
            }
        }
    }

    /// [`Daily::check_bound`], its refusal the reason alone, as [`Daily::allow`] gives it.
    pub fn allow_bound(&self, left: Option<f64>) -> Result<Option<f64>, String> {
        self.check_bound(left).map_err(|refusal| refusal.reason)
    }

    /// [`Daily::allow`], its refusal saying whether a ceiling was reached.
    pub fn check(&self, left: f64) -> Result<f64, Refusal> {
        if self.ceiling.is_none() && self.token_ceiling.is_none() {
            return Ok(left);
        }
        if let Some(e) = &self.unrecorded {
            return Err(Refusal::not_reached(format!(
                "{}: the spend of the day was not recorded: {e}",
                self.named()
            )));
        }
        if let (Some(cap), true) = (self.token_ceiling, self.uncounted) {
            return Err(Refusal::not_reached(format!(
                "daily ceiling of {cap} tokens: {UNCOUNTED}"
            )));
        }
        let today = day(now_ms());
        let spent = spent(self.layout, &today)
            .map_err(|e| Refusal::not_reached(format!("{}: {e}", self.named())))?;
        let mut allowed = left;
        if let Some(cap) = self.ceiling {
            let ceiling = cap as f64 / MICROS;
            let remaining = cap - spent.micros;
            if remaining <= 0 {
                let spent = spent.micros as f64 / MICROS;
                return Err(Refusal {
                    reason: format!(
                        "daily ceiling of {ceiling} USD reached: {spent:.4} USD spent on {today} \
                         (UTC)"
                    ),
                    reached: true,
                });
            }
            allowed = allowed.min(remaining as f64 / MICROS);
        }
        if let Some(cap) = self.token_ceiling {
            if spent.tokens >= cap {
                return Err(Refusal {
                    reason: format!(
                        "daily ceiling of {cap} tokens reached: {} tokens used on {today} (UTC)",
                        spent.tokens
                    ),
                    reached: true,
                });
            }
        }
        Ok(allowed)
    }

    /// Adds what one answer cost to the spend of today, as [`Daily::add_answer`] does, and none
    /// of its tokens.
    pub fn add(&mut self, cost_usd: Option<f64>) {
        self.record_answer(cost_usd, None);
    }

    /// Adds what one answer cost and the tokens its `usage` reported ([`tokens`]) to the spend of
    /// today, under the home's lock, the cost rounded to the micro-dollar; an answer with no cost,
    /// or a negative one, adds no dollars, and one whose tokens could not be counted no tokens. A
    /// spend that cannot be recorded is said on stderr and, with a ceiling, allows no further
    /// call; so does, with a ceiling in tokens, an answer whose tokens could not be counted
    /// ([`Daily::allow`]). Behind a model gateway the cost adds to `uncounted_usd`, not to
    /// `spent_usd`.
    pub fn add_answer(&mut self, cost_usd: Option<f64>, usage: &Usage) {
        let tokens = tokens(usage);
        if tokens.is_none() {
            self.uncounted = true;
        }
        self.record_answer(cost_usd, tokens);
    }

    fn record_answer(&mut self, cost_usd: Option<f64>, tokens: Option<i64>) {
        if cost_usd.is_none() && tokens.is_none() {
            return;
        }
        if let Err(e) = self.record(cost_usd.unwrap_or(0.0), tokens) {
            eprintln!("cortex: the spend of the day was not recorded: {e}");
            self.unrecorded = Some(e);
        }
    }

    fn record(&self, cost: f64, tokens: Option<i64>) -> Result<(), String> {
        let _lock = match self.home {
            Some(home) => Some(
                home.lock()
                    .map_err(|e| format!("cannot lock {}: {e}", home.root.display()))?,
            ),
            None => None,
        };
        let today = day(now_ms());
        let before = spent(self.layout, &today)?;
        let cost = micros(cost.max(0.0));
        // Behind a model gateway the cost is recorded as reported, in `uncounted_usd`, and adds
        // nothing to `spent_usd`; `uncounted_usd` is written from the day's first such answer and
        // kept with the day.
        let (spent, uncounted) = if self.counts_dollars {
            (before.micros + cost, before.uncounted)
        } else {
            (
                before.micros,
                Some(before.uncounted.unwrap_or(0).saturating_add(cost)),
            )
        };
        let usd = (spent as f64 / MICROS).to_string();
        let mut document = json!({"format": FORMAT, "day": today, "spent_usd": usd});
        if let Some(uncounted) = uncounted {
            document["uncounted_usd"] = json!((uncounted as f64 / MICROS).to_string());
        }
        // `spent_tokens` is written from the day's first answer that reported a `usage`.
        if before.counts_tokens || tokens.is_some() {
            let sum = before
                .tokens
                .checked_add(tokens.unwrap_or(0).max(0))
                .ok_or_else(|| {
                    format!(
                        "{}: the tokens of {today} would pass {} and cannot be summed",
                        path(self.layout).display(),
                        i64::MAX
                    )
                })?;
            document["spent_tokens"] = json!(sum);
        }
        write_atomic(
            &path(self.layout),
            serde_json::to_string_pretty(&document)
                .expect("plain JSON")
                .as_bytes(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(ceiling: Option<&str>) -> m::ModelSpec {
        m::ModelSpec {
            model: "m".into(),
            backend: None,
            budget_usd: Some(cortex_model::primitives::Decimal("1".into())),
            daily_ceiling_usd: ceiling.map(|c| cortex_model::primitives::Decimal(c.into())),
            budget_tokens: None,
            daily_ceiling_tokens: None,
            gateway: None,
            timeout_s: 60,
            instructions: None,
        }
    }

    #[test]
    fn behind_a_gateway_the_cost_is_uncounted_bounds_nothing_and_is_kept_with_the_day() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let mut gateway = model(Some("1"));
        gateway.gateway = Some(m::ModelGateway {
            base_url: "http://gw.test".into(),
            token_env: "GW".into(),
            max_context_tokens: None,
        });
        // A frozen spec from before the token budget was required behind a gateway counts
        // dollars as before.
        assert!(counts_dollars(&gateway));
        assert_eq!(dollar_budget(&gateway), Some(1.0));
        gateway.budget_tokens = Some(1000);
        assert!(!counts_dollars(&gateway));
        assert_eq!(dollar_budget(&gateway), None);
        assert_eq!(dollar_budget(&model(None)), Some(1.0));
        let mut daily = Daily::new(&layout, &gateway, None);
        daily.add(Some(5.0));
        daily.add(Some(0.25));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.0));
        assert_eq!(
            daily.allow(f64::INFINITY),
            Ok(f64::INFINITY),
            "no dollar ceiling"
        );
        assert_eq!(
            daily.check_bound(None),
            Ok(None),
            "no dollar bound reaches claude"
        );
        let mut plain_model = model(None);
        plain_model.budget_tokens = Some(1000);
        let plain_daily = Daily::new(&layout, &plain_model, None);
        assert_eq!(plain_daily.check_bound(Some(0.5)), Ok(Some(0.5)));
        let read = || -> Value {
            serde_json::from_str(&std::fs::read_to_string(path(&layout)).unwrap()).unwrap()
        };
        assert_eq!(read()["uncounted_usd"], "5.25");
        // The same day without a gateway (an update dropped it): counted again, the uncounted
        // sum kept.
        let mut plain = Daily::new(&layout, &model(Some("1")), None);
        plain.add(Some(0.5));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.5));
        assert_eq!(read()["uncounted_usd"], "5.25");
        // An unreadable `uncounted_usd` fails closed, as `spent_usd` does.
        std::fs::write(
            path(&layout),
            json!({"format": FORMAT, "day": day(now_ms()), "spent_usd": "0",
                "uncounted_usd": "-1"})
            .to_string(),
        )
        .unwrap();
        assert!(spent_on(&layout, &day(now_ms())).is_err());
    }

    #[test]
    fn the_day_is_the_utc_calendar_day() {
        assert_eq!(day(0), "1970-01-01");
        // 2026-10-14T23:59:59.999Z and the next millisecond.
        assert_eq!(day(1_792_022_399_999), "2026-10-14");
        assert_eq!(day(1_792_022_400_000), "2026-10-15");
    }

    #[test]
    fn a_spend_of_another_day_counts_as_nothing_and_today_adds_up() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        std::fs::write(
            path(&layout),
            json!({"format": FORMAT, "day": "2000-01-01", "spent_usd": "100"}).to_string(),
        )
        .unwrap();
        let model = model(Some("1"));
        let mut daily = Daily::new(&layout, &model, None);
        assert_eq!(
            daily.allow(5.0),
            Ok(1.0),
            "yesterday's spend counts as nothing"
        );
        daily.add(Some(0.25));
        daily.add(None);
        daily.add(Some(0.5));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.75));
        assert_eq!(daily.allow(5.0), Ok(0.25));
        assert_eq!(daily.allow(0.1), Ok(0.1), "the lesser of the two");
        daily.add(Some(0.25));
        let why = daily.allow(5.0).unwrap_err();
        assert!(why.starts_with("daily ceiling of 1 USD reached"), "{why}");
    }

    #[test]
    fn tokens_add_up_to_the_ceiling_and_an_uncounted_answer_allows_no_further_call() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let usage = |input| Usage {
            input_tokens: Some(input),
            cache_creation_input_tokens: Some(5),
            cache_read_input_tokens: Some(5),
            output_tokens: Some(10),
        };
        let mut model = model(None);
        model.daily_ceiling_tokens = Some(100);
        let mut daily = Daily::new(&layout, &model, None);
        daily.add_answer(None, &usage(30));
        assert_eq!(tokens_on(&layout, &day(now_ms())), Ok(50));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.0));
        assert_eq!(daily.allow(5.0), Ok(5.0), "no dollar ceiling to lessen it");
        daily.add_answer(Some(0.5), &usage(30));
        assert_eq!(tokens_on(&layout, &day(now_ms())), Ok(100));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.5));
        let why = daily.allow(5.0).unwrap_err();
        assert!(
            why.starts_with("daily ceiling of 100 tokens reached"),
            "{why}"
        );

        model.daily_ceiling_tokens = Some(1000);
        let mut daily = Daily::new(&layout, &model, None);
        daily.add_answer(Some(0.25), &Usage::UNKNOWN);
        assert_eq!(tokens_on(&layout, &day(now_ms())), Ok(100), "adds nothing");
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(0.75));
        let why = daily.allow(5.0).unwrap_err();
        assert!(why.contains("could not be counted"), "{why}");
    }

    #[test]
    fn without_a_ceiling_in_tokens_an_uncounted_answer_bounds_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let model = model(Some("1"));
        let mut daily = Daily::new(&layout, &model, None);
        daily.add_answer(Some(0.25), &Usage::UNKNOWN);
        assert_eq!(daily.allow(5.0), Ok(0.75));
        let text = std::fs::read_to_string(path(&layout)).unwrap();
        assert!(!text.contains("spent_tokens"), "{text}");
    }

    #[test]
    fn without_a_ceiling_the_spend_is_recorded_and_bounds_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let model = model(None);
        let mut daily = Daily::new(&layout, &model, None);
        daily.add(Some(7.0));
        assert_eq!(spent_on(&layout, &day(now_ms())), Ok(7.0));
        assert_eq!(daily.allow(0.5), Ok(0.5));
    }

    fn usage(input: i64, output: i64) -> Usage {
        Usage {
            input_tokens: Some(input),
            cache_creation_input_tokens: Some(0),
            cache_read_input_tokens: Some(0),
            output_tokens: Some(output),
        }
    }

    #[test]
    fn a_count_below_0_or_a_sum_past_i64_is_not_counted() {
        assert_eq!(tokens(&usage(10, 5)), Some(15));
        assert_eq!(tokens(&usage(1500, -1900)), None);
        assert_eq!(tokens(&usage(-1, 0)), None);
        assert_eq!(tokens(&usage(i64::MAX - 10, 100)), None);
        assert_eq!(tokens(&usage(i64::MAX, 0)), Some(i64::MAX));
    }

    #[test]
    fn a_run_token_budget_stops_at_its_cap_and_on_an_uncounted_or_unsummable_answer() {
        let mut model = model(None);
        assert_eq!(TokenBudget::new(&model).check(), Ok(()), "no budget");
        model.budget_tokens = Some(100);
        let mut budget = TokenBudget::new(&model);
        budget.add(&usage(60, 0));
        assert_eq!(budget.check(), Ok(()));
        budget.add(&usage(40, 0));
        assert_eq!(
            budget.check(),
            Err("token budget of 100 tokens spent: 100 tokens used".into())
        );

        for answer in [usage(0, -5), Usage::UNKNOWN] {
            let mut budget = TokenBudget::new(&model);
            budget.add(&answer);
            let why = budget.check().unwrap_err();
            assert!(why.contains("could not be counted"), "{why}");
        }
        let mut budget = TokenBudget::new(&model);
        budget.add(&usage(50, 0));
        budget.add(&usage(i64::MAX - 10, 0));
        let why = budget.check().unwrap_err();
        assert!(
            why.contains("could not be counted"),
            "a sum past i64: {why}"
        );
    }

    #[test]
    fn only_a_reached_ceiling_is_reached_and_a_day_past_i64_is_not_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let mut model = model(None);
        model.daily_ceiling_tokens = Some(i64::MAX);
        let mut daily = Daily::new(&layout, &model, None);
        daily.add_answer(None, &usage(0, -1));
        let refusal = daily.check(5.0).unwrap_err();
        assert!(!refusal.reached, "{refusal:?}");
        assert!(
            refusal.reason.contains("could not be counted"),
            "{refusal:?}"
        );

        std::fs::write(
            path(&layout),
            json!({"format": FORMAT, "day": day(now_ms()), "spent_usd": "0",
                "spent_tokens": i64::MAX - 10})
            .to_string(),
        )
        .unwrap();
        let mut daily = Daily::new(&layout, &model, None);
        assert_eq!(daily.check(5.0), Ok(5.0));
        daily.add_answer(None, &usage(100, 0));
        assert_eq!(
            tokens_on(&layout, &day(now_ms())),
            Ok(i64::MAX - 10),
            "a sum past i64 is not written"
        );
        let refusal = daily.check(5.0).unwrap_err();
        assert!(!refusal.reached, "{refusal:?}");
        assert!(refusal.reason.contains("not recorded"), "{refusal:?}");

        model.daily_ceiling_tokens = Some(10);
        let refusal = Daily::new(&layout, &model, None).check(5.0).unwrap_err();
        assert!(refusal.reached, "{refusal:?}");
    }
}

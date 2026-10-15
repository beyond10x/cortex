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
/// together. `None` when the answer did not report each of them: its tokens cannot be counted.
pub fn tokens(usage: &Usage) -> Option<i64> {
    Some(usage.all_input()? + usage.output_tokens?)
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
}

/// [`spent_on`] and [`tokens_on`] together. A `spent_usd` of today that is not a finite number of
/// 0 or more, or a `spent_tokens` that is not a whole number of 0 or more, is an error, so a
/// ceiling it would lift fails closed.
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
    let micros = value["spent_usd"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(micros)
        .ok_or_else(|| format!("{}: spent_usd is not a number of 0 or more", path.display()))?;
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
    /// An answer reported no `usage`: with a ceiling in tokens, no further call is allowed.
    uncounted: bool,
}

impl<'a> Daily<'a> {
    /// The ceilings `model` names, for a command that holds the home's lock (`home` `None`) or
    /// takes it for each record (`home` the home).
    pub fn new(layout: &'a Layout, model: &m::ModelSpec, home: Option<&'a Home>) -> Self {
        Self {
            layout,
            ceiling: model
                .daily_ceiling_usd
                .as_ref()
                .and_then(|c| c.0.parse::<f64>().ok())
                .map(micros),
            token_ceiling: model.daily_ceiling_tokens,
            home,
            unrecorded: None,
            uncounted: false,
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
        if self.ceiling.is_none() && self.token_ceiling.is_none() {
            return Ok(left);
        }
        if let Some(e) = &self.unrecorded {
            return Err(format!(
                "{}: the spend of the day was not recorded: {e}",
                self.named()
            ));
        }
        if let (Some(cap), true) = (self.token_ceiling, self.uncounted) {
            return Err(format!(
                "daily ceiling of {cap} tokens: the tokens of an answer could not be counted: \
                 it reported no usage"
            ));
        }
        let today = day(now_ms());
        let spent = spent(self.layout, &today).map_err(|e| format!("{}: {e}", self.named()))?;
        let mut allowed = left;
        if let Some(cap) = self.ceiling {
            let ceiling = cap as f64 / MICROS;
            let remaining = cap - spent.micros;
            if remaining <= 0 {
                let spent = spent.micros as f64 / MICROS;
                return Err(format!(
                    "daily ceiling of {ceiling} USD reached: {spent:.4} USD spent on {today} (UTC)"
                ));
            }
            allowed = allowed.min(remaining as f64 / MICROS);
        }
        if let Some(cap) = self.token_ceiling {
            if spent.tokens >= cap {
                return Err(format!(
                    "daily ceiling of {cap} tokens reached: {} tokens used on {today} (UTC)",
                    spent.tokens
                ));
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
    /// or a negative one, adds no dollars, and one with no `usage` no tokens. A spend that cannot
    /// be recorded is said on stderr and, with a ceiling, allows no further call; so does, with a
    /// ceiling in tokens, an answer with no `usage` ([`Daily::allow`]).
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
        let spent = before.micros + micros(cost.max(0.0));
        let usd = (spent as f64 / MICROS).to_string();
        let mut document = json!({"format": FORMAT, "day": today, "spent_usd": usd});
        // `spent_tokens` is written from the day's first answer that reported a `usage`.
        if before.counts_tokens || tokens.is_some() {
            document["spent_tokens"] = json!(before.tokens + tokens.unwrap_or(0).max(0));
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
            budget_usd: cortex_model::primitives::Decimal("1".into()),
            daily_ceiling_usd: ceiling.map(|c| cortex_model::primitives::Decimal(c.into())),
            budget_tokens: None,
            daily_ceiling_tokens: None,
            gateway: None,
            timeout_s: 60,
            instructions: None,
        }
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
}

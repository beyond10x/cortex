//! The spend of the day of one instance (`cortex.instance.DailySpend`): `spend.json` in the
//! instance directory, the sum of the costs its model answers stated on one UTC calendar day, and
//! the daily ceiling (`model.daily_ceiling_usd`) every model call of the instance is held to.

use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::{json, Value};

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

/// Micro-dollars in one US dollar. The spend of the day is summed in whole micro-dollars, so a sum
/// of decimal costs reaches a ceiling it equals (0.7 + 0.1 USD is 0.8 USD, not 0.7999…).
const MICROS: f64 = 1_000_000.0;

/// `usd` in whole micro-dollars, rounded to the nearest.
fn micros(usd: f64) -> i64 {
    (usd * MICROS).round() as i64
}

/// What `spend.json` says was spent on `day`: `0` when there is no file or it counts another day.
pub fn spent_on(layout: &Layout, day: &str) -> Result<f64, String> {
    spent_micros(layout, day).map(|m| m as f64 / MICROS)
}

/// [`spent_on`] in whole micro-dollars. A `spent_usd` of today that is not a finite number of 0
/// or more is an error, so a ceiling it would lift fails closed.
fn spent_micros(layout: &Layout, day: &str) -> Result<i64, String> {
    let path = path(layout);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{} is not JSON: {e}", path.display()))?;
    if value["format"] != FORMAT {
        return Err(format!("{} is not a {FORMAT} document", path.display()));
    }
    if value["day"].as_str() != Some(day) {
        return Ok(0);
    }
    value["spent_usd"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(micros)
        .ok_or_else(|| format!("{}: spent_usd is not a number of 0 or more", path.display()))
}

/// The daily ceiling of one command's model calls, and the record of what they spend.
pub struct Daily<'a> {
    layout: &'a Layout,
    /// The ceiling in whole micro-dollars, as the spend of the day is summed.
    ceiling: Option<i64>,
    /// The home whose lock [`Daily::add`] takes; `None` when the command holds it already.
    home: Option<&'a Home>,
    /// Why an answer's cost could not be recorded: with a ceiling, no further call is allowed.
    unrecorded: Option<String>,
}

impl<'a> Daily<'a> {
    /// The ceiling `model` names, for a command that holds the home's lock (`home` `None`) or
    /// takes it for each record (`home` the home).
    pub fn new(layout: &'a Layout, model: &m::ModelSpec, home: Option<&'a Home>) -> Self {
        Self {
            layout,
            ceiling: model
                .daily_ceiling_usd
                .as_ref()
                .and_then(|c| c.0.parse::<f64>().ok())
                .map(micros),
            home,
            unrecorded: None,
        }
    }

    /// What the next model call may spend: the lesser of `left`, what remains of the command's
    /// own budget, and what the ceiling leaves of today. An error, naming the daily ceiling, when
    /// it leaves nothing or what was spent today cannot be known.
    pub fn allow(&self, left: f64) -> Result<f64, String> {
        let Some(cap) = self.ceiling else {
            return Ok(left);
        };
        let ceiling = cap as f64 / MICROS;
        if let Some(e) = &self.unrecorded {
            return Err(format!(
                "daily ceiling of {ceiling} USD: the spend of the day was not recorded: {e}"
            ));
        }
        let today = day(now_ms());
        let spent = spent_micros(self.layout, &today)
            .map_err(|e| format!("daily ceiling of {ceiling} USD: {e}"))?;
        let remaining = cap - spent;
        if remaining <= 0 {
            let spent = spent as f64 / MICROS;
            return Err(format!(
                "daily ceiling of {ceiling} USD reached: {spent:.4} USD spent on {today} (UTC)"
            ));
        }
        Ok(left.min(remaining as f64 / MICROS))
    }

    /// Adds what one answer cost to the spend of today, under the home's lock, rounded to the
    /// micro-dollar; an answer with no cost, or a negative one, adds nothing. A cost that cannot be recorded is said on stderr and, with a
    /// ceiling, allows no further call ([`Daily::allow`]).
    pub fn add(&mut self, cost_usd: Option<f64>) {
        let Some(cost) = cost_usd else {
            return;
        };
        if let Err(e) = self.record(cost) {
            eprintln!("cortex: the spend of the day was not recorded: {e}");
            self.unrecorded = Some(e);
        }
    }

    fn record(&self, cost: f64) -> Result<(), String> {
        let _lock = match self.home {
            Some(home) => Some(
                home.lock()
                    .map_err(|e| format!("cannot lock {}: {e}", home.root.display()))?,
            ),
            None => None,
        };
        let today = day(now_ms());
        let spent = spent_micros(self.layout, &today)? + micros(cost.max(0.0));
        let usd = (spent as f64 / MICROS).to_string();
        let document = json!({"format": FORMAT, "day": today, "spent_usd": usd});
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

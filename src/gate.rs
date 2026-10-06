//! The run gate, the spec file's `gate`: checks a source run is held to once it applied
//! something. Each check names a measure and a `min`, a `max` or both; a measure is one of the run
//! report's ([`REPORT_MEASURES`]) or, by its dotted path, a number of the `ekr.store-quality/1`
//! document `ekr quality` prints for the store after the run (`assertions.active`). A check fails
//! when its measure is below `min`, above `max`, or cannot be read; both are compared at 4
//! decimal places, the precision cortex prints a cost at. `run.rs` then restores the snapshot
//! taken before the run.

use cortex_model::instance as m;
use serde_json::{json, Value};

use crate::ekr::Store;
use crate::run::Report;

/// The measures of the run report `cortex run` prints, read before `ekr quality` is asked.
pub const REPORT_MEASURES: [&str; 8] = [
    "documents_new",
    "documents_applied",
    "facts_refused",
    "parts_rejected",
    "masked",
    "truncated",
    "unrestored",
    "cost_usd",
];

/// A check the run failed.
#[derive(Debug, Clone, PartialEq)]
pub struct Failed {
    pub measure: String,
    /// The measure's value; `None` when it could not be read.
    pub value: Option<f64>,
    pub min: Option<String>,
    pub max: Option<String>,
    /// Why the measure could not be read, when it could not.
    pub unread: Option<String>,
}

impl Failed {
    /// `facts_refused = 1 (max 0)`, or why the measure could not be read.
    pub fn describe(&self) -> String {
        let bounds: Vec<String> = [("min", &self.min), ("max", &self.max)]
            .into_iter()
            .filter_map(|(name, bound)| bound.as_ref().map(|b| format!("{name} {b}")))
            .collect();
        match (&self.value, &self.unread) {
            (Some(value), _) => format!(
                "{} = {} ({})",
                self.measure,
                number(*value),
                bounds.join(", ")
            ),
            (None, why) => format!(
                "{} cannot be read ({}): {}",
                self.measure,
                bounds.join(", "),
                why.as_deref().unwrap_or("no value")
            ),
        }
    }

    /// The failed check as `cortex.log` records it.
    pub fn json(&self) -> Value {
        let mut line = json!({"measure": self.measure, "min": self.min, "max": self.max});
        match self.value {
            Some(v) => line["value"] = number_json(v),
            None => line["unread"] = json!(self.unread),
        }
        line
    }
}

/// `value` as written in a report: an integer without a fraction.
fn number(value: f64) -> String {
    match integer(value) {
        Some(i) => i.to_string(),
        None => value.to_string(),
    }
}

fn number_json(value: f64) -> Value {
    match integer(value) {
        Some(i) => json!(i),
        None => json!(value),
    }
}

fn integer(value: f64) -> Option<i64> {
    (value.fract() == 0.0 && value.abs() < 9.0e15).then_some(value as i64)
}

/// A run-report measure: `None` when `name` is none of [`REPORT_MEASURES`], `Some(None)` when it
/// is one with no value (a cost no answer carried).
fn report_measure(report: &Report, name: &str) -> Option<Option<f64>> {
    Some(Some(match name {
        "documents_new" => report.documents_new as f64,
        "documents_applied" => report.documents_applied as f64,
        "facts_refused" => report.facts_refused as f64,
        "parts_rejected" => report.parts_rejected as f64,
        "masked" => report.masked as f64,
        "truncated" => report.truncated as f64,
        "unrestored" => report.unrestored as f64,
        "cost_usd" => return Some(report.cost_usd),
        _ => return None,
    }))
}

/// The number at the dotted `path` of `doc`.
fn at_path(doc: &Value, path: &str) -> Option<f64> {
    path.split('.')
        .try_fold(doc, |node, key| node.get(key))
        .and_then(Value::as_f64)
}

/// `ekr quality` for `store` at its newest revision.
pub fn quality(store: &Store) -> Result<Value, String> {
    let out = store
        .command(&[std::ffi::OsStr::new("quality")])?
        .output()
        .map_err(|e| format!("cannot run ekr quality: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ekr quality failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("ekr quality answered no JSON: {e}"))
}

/// Measures and bounds are compared at the precision cortex prints a cost, 4 decimal places, as
/// integers scaled by this.
const SCALE: i128 = 10_000;
const PLACES: usize = 4;

/// A measure rounded to 4 decimal places, scaled by [`SCALE`].
fn scaled_value(value: f64) -> Result<i128, String> {
    let scaled = (value * SCALE as f64).round();
    if !scaled.is_finite() || scaled.abs() >= 1e36 {
        return Err(format!("{value} is beyond what cortex compares"));
    }
    Ok(scaled as i128)
}

/// A decimal bound as the spec writes it (`-?(0|[1-9][0-9]*)(\.[0-9]+)?`), read exactly and
/// rounded half away from zero to 4 decimal places, scaled by [`SCALE`].
fn scaled_bound(bound: &str) -> Result<i128, String> {
    let not = || format!("{bound:?} is not a decimal cortex can compare");
    let (negative, digits) = match bound.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, bound),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() || !(whole.bytes().chain(fraction.bytes())).all(|b| b.is_ascii_digit()) {
        return Err(not());
    }
    let whole: i128 = whole.parse().map_err(|_| not())?;
    let kept = format!("{fraction:0<width$}", width = PLACES + 1);
    let places: i128 = kept[..PLACES].parse().map_err(|_| not())?;
    let up = i128::from(kept.as_bytes()[PLACES] >= b'5');
    let magnitude = whole
        .checked_mul(SCALE)
        .and_then(|w| w.checked_add(places + up))
        .ok_or_else(not)?;
    Ok(if negative { -magnitude } else { magnitude })
}

/// The checks of `gate` the run failed, in the gate's order. `ekr quality` is asked through
/// `quality` once, and only when a check names a measure the run report does not have.
pub fn evaluate(
    gate: &m::RunGate,
    report: &Report,
    quality: impl FnOnce() -> Result<Value, String>,
) -> Vec<Failed> {
    let mut quality = Some(quality);
    let mut doc: Option<Result<Value, String>> = None;
    let mut failed = Vec::new();
    for check in &gate.checks {
        let min = check.min.as_ref().map(|d| d.0.clone());
        let max = check.max.as_ref().map(|d| d.0.clone());
        let value: Result<f64, String> = match report_measure(report, &check.measure) {
            Some(Some(v)) => Ok(v),
            Some(None) => Err("no answer of the run carried a cost".into()),
            None => {
                let doc = doc.get_or_insert_with(|| (quality.take().expect("asked once"))());
                match doc {
                    Ok(doc) => at_path(doc, &check.measure).ok_or_else(|| {
                        format!(
                            "neither the run report nor ekr quality has a number named {:?}",
                            check.measure
                        )
                    }),
                    Err(e) => Err(e.clone()),
                }
            }
        };
        let bound = |b: &Option<String>| b.as_deref().map(scaled_bound).transpose();
        let outcome = value.and_then(|v| Ok((scaled_value(v)?, bound(&min)?, bound(&max)?)));
        let (value, unread) = match outcome {
            Ok((v, lo, hi)) => {
                if lo.is_some_and(|lo| v < lo) || hi.is_some_and(|hi| v > hi) {
                    (Some(v as f64 / SCALE as f64), None)
                } else {
                    continue;
                }
            }
            Err(why) => (None, Some(why)),
        };
        failed.push(Failed {
            measure: check.measure.clone(),
            value,
            min,
            max,
            unread,
        });
    }
    failed
}

/// Why a run failed its gate, naming each failed check's measure and value.
pub fn reason(failed: &[Failed]) -> String {
    format!(
        "the run failed its gate: {}",
        failed
            .iter()
            .map(Failed::describe)
            .collect::<Vec<_>>()
            .join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_model::primitives::Decimal;

    fn check(measure: &str, min: Option<&str>, max: Option<&str>) -> m::GateCheck {
        m::GateCheck {
            measure: measure.into(),
            min: min.map(|v| Decimal(v.into())),
            max: max.map(|v| Decimal(v.into())),
        }
    }

    fn gate(checks: Vec<m::GateCheck>) -> m::RunGate {
        m::RunGate { checks }
    }

    fn no_quality() -> Result<Value, String> {
        panic!("ekr quality was asked for a run-report measure")
    }

    #[test]
    fn a_report_measure_beyond_a_bound_fails_and_one_within_passes() {
        let report = Report {
            facts_refused: 1,
            documents_applied: 2,
            ..Report::default()
        };
        let checks = gate(vec![
            check("facts_refused", None, Some("0")),
            check("facts_refused", None, Some("1")),
            check("documents_applied", Some("3"), None),
            check("documents_applied", Some("1"), Some("2")),
        ]);
        let failed = evaluate(&checks, &report, no_quality);
        let named: Vec<String> = failed.iter().map(Failed::describe).collect();
        assert_eq!(
            named,
            ["facts_refused = 1 (max 0)", "documents_applied = 2 (min 3)"]
        );
        assert_eq!(failed[0].json()["value"], 1);
        assert_eq!(
            reason(&failed),
            "the run failed its gate: facts_refused = 1 (max 0); documents_applied = 2 (min 3)"
        );
    }

    #[test]
    fn a_decimal_bound_and_an_unknown_cost_are_held() {
        let costed = Report {
            cost_usd: Some(0.25),
            ..Report::default()
        };
        let checks = gate(vec![check("cost_usd", None, Some("0.2"))]);
        assert_eq!(
            evaluate(&checks, &costed, no_quality)[0].describe(),
            "cost_usd = 0.25 (max 0.2)"
        );
        // A cost no answer carried is not 0: a check on it fails rather than passing unread.
        let uncosted = Report {
            cost_usd: None,
            ..Report::default()
        };
        let failed = evaluate(&checks, &uncosted, no_quality);
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].value, None);
    }

    #[test]
    fn a_quality_measure_is_read_by_its_path_with_one_ekr_call() {
        let doc = json!({"assertions": {"active": 4, "with_evidence": 4}, "sharing_nodes": 0});
        let mut calls = 0;
        let checks = gate(vec![
            check("assertions.active", None, Some("3")),
            check("sharing_nodes", None, Some("0")),
            check("assertions.with_evidence", Some("1"), None),
        ]);
        let failed = evaluate(&checks, &Report::default(), || {
            calls += 1;
            Ok(doc)
        });
        assert_eq!(calls, 1);
        let named: Vec<String> = failed.iter().map(Failed::describe).collect();
        assert_eq!(named, ["assertions.active = 4 (max 3)"]);
    }

    #[test]
    fn a_measure_that_cannot_be_read_fails_its_check() {
        // A misspelt measure, or an `ekr quality` that fails, fails the gate rather than passing.
        let doc = json!({"assertions": {"active": 4}});
        let checks = gate(vec![
            check("facts_refusd", None, Some("0")),
            check("assertions", None, Some("0")),
        ]);
        let failed = evaluate(&checks, &Report::default(), || Ok(doc));
        assert_eq!(failed.len(), 2, "{failed:?}");
        assert!(failed.iter().all(|f| f.value.is_none()));
        assert!(failed[0].describe().contains("facts_refusd"));

        let failed = evaluate(
            &gate(vec![check("assertions.active", None, Some("9"))]),
            &Report::default(),
            || Err("ekr quality failed: boom".into()),
        );
        assert_eq!(
            failed[0].unread.as_deref(),
            Some("ekr quality failed: boom")
        );
        assert!(failed[0].json()["unread"].is_string());
    }

    #[test]
    fn values_and_bounds_compare_at_the_4_decimals_cortex_prints() {
        let at = |cost: f64, max: &str| {
            let report = Report {
                cost_usd: Some(cost),
                ..Report::default()
            };
            evaluate(
                &gate(vec![check("cost_usd", None, Some(max))]),
                &report,
                no_quality,
            )
        };
        assert!(at(0.1 + 0.2, "0.3").is_empty());
        assert!(at(0.30004, "0.3").is_empty(), "0.30004 prints as 0.3000");
        assert!(at(0.3, "0.30000000001").is_empty());
        assert_eq!(
            at(0.3001, "0.3")[0].describe(),
            "cost_usd = 0.3001 (max 0.3)"
        );
        assert_eq!(
            at(0.3, "-0.0001")[0].describe(),
            "cost_usd = 0.3 (max -0.0001)"
        );
        // A bound beyond what cortex can scale fails its check rather than passing.
        let huge = at(0.3, "99999999999999999999999999999999999999999");
        assert_eq!(huge.len(), 1);
        assert_eq!(huge[0].value, None);
    }

    #[test]
    fn a_gate_with_no_checks_passes_without_asking_ekr() {
        assert!(evaluate(&gate(vec![]), &Report::default(), no_quality).is_empty());
    }
}

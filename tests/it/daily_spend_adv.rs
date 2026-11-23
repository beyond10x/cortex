//! Adversary, `story:daily-instance-budget`: the spend of the day (`spend.json`) and the
//! allowance [`Daily::allow`] gives the next model call, driven through the library.

use cortex_cli::instance::Layout;
use cortex_cli::spend::{self, Daily};
use cortex_model::instance as m;
use serde_json::json;

fn model(ceiling: &str) -> m::ModelSpec {
    m::ModelSpec {
        model: "m".into(),
        backend: None,
        budget_usd: Some(cortex_model::primitives::Decimal("5".into())),
        daily_ceiling_usd: Some(cortex_model::primitives::Decimal(ceiling.into())),
        budget_tokens: None,
        daily_ceiling_tokens: None,
        gateway: None,
        timeout_s: 60,
        instructions: None,
        batch_chars: None,
    }
}

fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    spend::day(now)
}

/// Two answers costing 0.7 and 0.1 USD spend 0.8 USD: a ceiling of 0.8 is reached and the next
/// call is refused. Summed in `f64` they make 0.7999999999999999, which leaves 1.1e-16 "of the
/// ceiling", and a call is allowed (and given `--max-budget-usd 0.0000`).
#[test]
fn adv_answers_of_0_7_and_0_1_reach_a_ceiling_of_0_8() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = Layout::new(tmp.path().to_path_buf());
    let model = model("0.8");
    let mut daily = Daily::new(&layout, &model, None);
    daily.add(Some(0.7));
    daily.add(Some(0.1));
    let allowed = daily.allow(5.0);
    assert!(
        allowed.is_err(),
        "0.7 + 0.1 USD spent of a ceiling of 0.8 USD still allows a call of {allowed:?} USD"
    );
}

/// A hand-edited or corrupt `spend.json` holding a negative spend for today fails closed, as one
/// that is not a number does: it does not lift the ceiling above what the spec names.
#[test]
fn adv_a_negative_spend_of_the_day_does_not_lift_the_ceiling() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = Layout::new(tmp.path().to_path_buf());
    std::fs::write(
        spend::path(&layout),
        json!({"format": spend::FORMAT, "day": today(), "spent_usd": "-100"}).to_string(),
    )
    .unwrap();
    let model = model("1");
    let daily = Daily::new(&layout, &model, None);
    let allowed = daily.allow(5.0);
    assert!(
        !matches!(allowed, Ok(usd) if usd > 1.0),
        "a ceiling of 1 USD allows a call of {allowed:?} USD"
    );
}

//! `cortex schema <instance> [--sample N] [--dry-run]` (`story:schema-emergence`): `ekr sample`
//! draws N facts with their evidence under the home's lock, a stand-in model answers each batch of
//! 20 with `cortex.instance.SchemaProposals`, and cortex applies each change EKR applies as a
//! schema transaction of its own, writing every proposal to `schema/<UTC stamp>/proposals.jsonl`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// An extraction stand-in: `n` products, each with its own `version`, all citing the first
/// evidence id of the batch, so the store holds `n` property facts besides the web pages'.
fn extract_products(w: &World, n: usize) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"id=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
entities=""
facts=""
i=1
while [ $i -le {n} ]; do
  entities="$entities{{\"node_type\":\"Product\",\"aliases\":[\"Product $i\"]}},"
  facts="$facts{{\"!Property\":{{\"subject\":{{\"node_type\":\"Product\",\"aliases\":[\"Product $i\"]}},\"property\":\"version\",\"value\":{{\"value_kind\":\"String\",\"value\":\"1.$i\"}},\"evidence\":[\"$id\"]}}}},"
  i=$((i+1))
done
entities=${{entities%,}}
facts=${{facts%,}}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Product","parents":[],"abstract_type":false,"properties":[{{"name":"version","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[$entities],"facts":[$facts]}}}}
EOF
"#
        ),
    );
}

/// The proposing model's stand-in: records its arguments, its prompt and whether
/// `ANTHROPIC_API_KEY` reached it, then answers `proposals-<call>.json`, or `proposals.json` when
/// there is none, at 0.02 USD. In the answer, `FACT0` is the first fact id of its own prompt and
/// `OTHER0` the first fact id of the first call's prompt. With the file `schema-error` it answers
/// an error instead. With the file `schema-no-cost`, which holds a call number counted from 0,
/// that call and every later one answer with no cost.
fn proposer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/schema-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/schema-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/schema-args.log"; echo >> "$R/schema-args.log"
env | grep -c '^ANTHROPIC_API_KEY=' >> "$R/schema-env.log"
prompt=$(cat)
printf '%s\n' "$prompt" > "$R/schema-prompt-$n.txt"
if [ -e "$R/schema-error" ]; then
  echo '{{"type":"result","subtype":"error_max_budget_usd","is_error":true,"total_cost_usd":0.02}}'
  exit 0
fi
fact=$(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
other=$(grep -oE '^=== Fact [0-9a-f-]{{36}}' "$R/schema-prompt-0.txt" | head -1 | cut -d' ' -f3)
file="$R/proposals-$n.json"
[ -e "$file" ] || file="$R/proposals.json"
proposals=$(sed -e "s/FACT0/$fact/g" -e "s/OTHER0/$other/g" "$file")
COST='"total_cost_usd":0.02,'
if [ -e "$R/schema-no-cost" ] && [ "$n" -ge "$(cat "$R/schema-no-cost")" ]; then COST=''; fi
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,$COST\"structured_output\":{{\"proposals\":$proposals}}}}"
"#,
            root = w.root.display()
        ),
    );
}

/// What the proposer answers on every call without a file of its own.
fn answer(w: &World, proposals: Value) {
    std::fs::write(w.root.join("proposals.json"), proposals.to_string()).unwrap();
}

/// An instance `name` whose store holds `products` product facts besides the web pages' facts,
/// with the proposer in place of the extraction stand-in.
fn instance(name: &str, products: usize) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    extract_products(&w, products);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    w
}

/// `ekr <args>` over the instance's store, its JSON answer.
fn ekr(w: &World, name: &str, args: &[&str]) -> Value {
    let dir = w.home.join("instances").join(name);
    let out = Command::new(common::ekr())
        .args(args)
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ekr {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// The property names `type_name` declares in `ontology`.
fn properties(ontology: &Value, type_name: &str) -> Vec<String> {
    ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(ontology["edge_types"].as_array().into_iter().flatten())
        .filter(|t| t["name"] == type_name)
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect()
}

fn type_names(ontology: &Value) -> Vec<String> {
    ["node_types", "edge_types"]
        .iter()
        .flat_map(|k| ontology[*k].as_array().cloned().unwrap_or_default())
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect()
}

fn schema_dirs(w: &World, name: &str) -> Vec<PathBuf> {
    let dir = w.home.join("instances").join(name).join("schema");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|d| d.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn lines(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("proposals.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn add_property(owner: &str, name: &str, kind: &str, fact: &str, evidence: &[&str]) -> Value {
    json!({
        "change": {"kind": "add_property", "value": {"owner": owner,
            "property": {"name": name, "value_kind": kind, "cardinality": "One"}}},
        "reason": format!("the evidence states a {name}"),
        "facts": [fact],
        "evidence": evidence,
    })
}

fn merge(fact: &str) -> Value {
    json!({
        "change": {"kind": "merge_types", "value": {"types": ["Product", "Gadget"], "into": "Thing"}},
        "reason": "a gadget is a product",
        "facts": [fact],
        "evidence": ["E1"],
    })
}

/// Acceptance 1.
#[test]
fn a_new_property_is_committed_as_a_schema_transaction_and_the_revision_before_keeps_the_old_schema(
) {
    let w = instance("grown", 3);
    let before = w.head("grown");
    answer(
        &w,
        json!([add_property(
            "Product",
            "founded",
            "Integer",
            "FACT0",
            &["E1"]
        )]),
    );
    let (code, out) = w.cortex(&["schema", "grown", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (
            d["proposed"].clone(),
            d["applied"].clone(),
            d["dropped"].clone()
        ),
        (json!(1), json!(1), json!(0)),
        "{out}"
    );
    assert_eq!(
        d["revision"], before,
        "the head the sample was drawn at: {out}"
    );
    assert_eq!(d["cost_usd"], "0.0200", "{out}");

    let dirs = schema_dirs(&w, "grown");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    assert_eq!(
        d["stamp"],
        dirs[0].file_name().unwrap().to_str().unwrap(),
        "{out}"
    );
    let lines = lines(&dirs[0]);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(line["format"], "cortex.schema-proposal/1", "{line}");
    assert_eq!(line["status"], "applied", "{line}");
    assert_eq!(line["change"]["kind"], "add_property", "{line}");
    assert_eq!(line["revision"], before + 1, "{line}");
    assert!(line["schema_version"].as_str().is_some(), "{line}");

    // The change cites its evidence: the fact it rests on and the evidence that fact and the
    // label `E1` name, each an id the store holds.
    let snapshot = ekr(&w, "grown", &["snapshot"]);
    let graph = &snapshot["graph"]["graph"];
    let facts: Vec<&str> = line["facts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(facts.len(), 1, "{line}");
    assert!(graph["assertions"].get(facts[0]).is_some(), "{line}");
    let evidence: Vec<&str> = line["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(!evidence.is_empty(), "{line}");
    for id in &evidence {
        assert!(
            graph["evidence"].get(*id).is_some(),
            "{id} is not in the store"
        );
    }
    let cited = &graph["assertions"][facts[0]]["evidence"];
    for id in cited.as_array().unwrap() {
        assert!(evidence.contains(&id.as_str().unwrap()), "{line} {cited}");
    }

    assert_eq!(w.head("grown"), before + 1);
    let then = ekr(&w, "grown", &["ontology", "--at", &before.to_string()]);
    let now = ekr(&w, "grown", &["ontology"]);
    assert!(
        !properties(&then, "Product").contains(&"founded".to_string()),
        "{then}"
    );
    assert!(
        properties(&now, "Product").contains(&"founded".to_string()),
        "{now}"
    );
    assert_eq!(
        now["schema_version_number"].as_i64(),
        then["schema_version_number"].as_i64().map(|n| n + 1),
        "one schema version more"
    );
    assert_eq!(now["schema_version"], line["schema_version"], "{line}");
}

/// Acceptance 2.
#[test]
fn a_merge_is_recorded_only_not_applied_and_not_an_error() {
    let w = instance("merged", 3);
    let before = w.head("merged");
    answer(&w, json!([merge("FACT0")]));
    let (code, out) = w.cortex(&["schema", "merged", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (
            d["proposed"].clone(),
            d["applied"].clone(),
            d["recorded_only"].clone()
        ),
        (json!(1), json!(0), json!(1)),
        "{out}"
    );
    let lines = lines(&schema_dirs(&w, "merged")[0]);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["status"], "recorded-only", "{lines:?}");
    assert_eq!(lines[0]["change"]["kind"], "merge_types", "{lines:?}");
    assert!(lines[0].get("revision").is_none(), "{lines:?}");
    assert_eq!(w.head("merged"), before, "nothing was committed");
}

/// Acceptance 3: a fact the batch does not hold, whether no batch holds it or another batch of
/// the same sample does, and an evidence label the batch does not have, drop the proposal.
#[test]
fn a_proposal_citing_a_fact_or_evidence_outside_its_batch_is_dropped() {
    let w = instance("dropping", 30);
    let forged = "00000000-0000-4000-8000-00000000beef";
    answer(
        &w,
        json!([
            add_property("Product", "founded", "Integer", "FACT0", &["E1"]),
            add_property("Product", "forged", "String", forged, &["E1"]),
            add_property("Product", "unlabelled", "String", "FACT0", &["E9"]),
        ]),
    );
    // The second batch's answer cites the first fact of the first batch.
    std::fs::write(
        w.root.join("proposals-1.json"),
        json!([add_property("Product", "borrowed", "String", "OTHER0", &[])]).to_string(),
    )
    .unwrap();
    let (code, out) = w.cortex(&["schema", "dropping", "--sample", "25"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(w.lines("schema-calls.log").len(), 2, "two batches: {out}");
    let d = &out["detail"];
    assert_eq!(
        (
            d["proposed"].clone(),
            d["applied"].clone(),
            d["dropped"].clone()
        ),
        (json!(1), json!(1), json!(3)),
        "{out}"
    );
    let lines = lines(&schema_dirs(&w, "dropping")[0]);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0]["change"]["value"]["property"]["name"], "founded",
        "{lines:?}"
    );
    let now = ekr(&w, "dropping", &["ontology"]);
    let props = properties(&now, "Product");
    for dropped in ["forged", "unlabelled", "borrowed"] {
        assert!(!props.contains(&dropped.to_string()), "{dropped}: {now}");
    }
}

/// Acceptance 4.
#[test]
fn a_dry_run_applies_nothing_and_writes_every_proposal() {
    let w = instance("dry", 3);
    let before = w.head("dry");
    let ontology = ekr(&w, "dry", &["ontology"]);
    answer(
        &w,
        json!([
            add_property("Product", "founded", "Integer", "FACT0", &["E1"]),
            merge("FACT0"),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "dry", "--sample", "5", "--dry-run"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("proposed")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (
            d["proposed"].clone(),
            d["applied"].clone(),
            d["refused"].clone()
        ),
        (json!(2), json!(0), json!(0)),
        "{out}"
    );
    assert_eq!(w.head("dry"), before, "a dry run commits nothing");
    assert_eq!(ekr(&w, "dry", &["ontology"]), ontology);
    let lines = lines(&schema_dirs(&w, "dry")[0]);
    let statuses: Vec<&str> = lines.iter().filter_map(|l| l["status"].as_str()).collect();
    assert_eq!(statuses, ["dry-run", "recorded-only"], "{lines:?}");
}

/// `story:events-carry-measurements`: `SchemaChangesProposed` carries `cost_usd`, in a dry run as
/// when applying: the sum while every answer is costed, and null as soon as one answer carried
/// no cost, never the sum of the costed ones and never 0.
#[test]
fn the_cost_is_null_as_soon_as_one_answer_carried_none() {
    let w = instance("costed", 30);
    answer(&w, json!([]));
    let (code, out) = w.cortex(&["schema", "costed", "--sample", "25", "--dry-run"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("proposed")),
        "{out}"
    );
    assert_eq!(out["detail"]["cost_usd"], "0.0400", "two calls: {out}");

    // Calls 2 and 3 are this dry run's two batches; the second carries no cost.
    std::fs::write(w.root.join("schema-no-cost"), "3").unwrap();
    let (code, out) = w.cortex(&["schema", "costed", "--sample", "25", "--dry-run"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("proposed")),
        "{out}"
    );
    assert_eq!(w.lines("schema-calls.log").len(), 4, "{out}");
    assert_eq!(out["detail"].get("cost_usd"), Some(&Value::Null), "{out}");

    let (code, out) = w.cortex(&["schema", "costed", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"].get("cost_usd"), Some(&Value::Null), "{out}");
}

/// Adversary, `story:events-carry-measurements`: with no fact to draw no model is asked, and
/// `SchemaChangesProposed` carries a cost of 0, not null, in a dry run as when applying. The case
/// `adv_an_empty_store_asks_no_model_and_applies_nothing` asserts the cost only when its outcome
/// is `applied`; this one asserts both outcomes.
#[test]
fn adv_an_empty_store_costs_nothing_in_a_dry_run_and_when_applying() {
    let w = World::new();
    let path = w.spec("empty", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    proposer(&w);
    answer(&w, json!([]));
    for (args, outcome) in [
        (
            &["schema", "empty", "--sample", "5", "--dry-run"][..],
            "proposed",
        ),
        (&["schema", "empty", "--sample", "5"][..], "applied"),
    ] {
        let (code, out) = w.cortex(args);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (0, Some(outcome)),
            "{args:?}: {out}"
        );
        assert_eq!(out["detail"]["proposed"], 0, "{args:?}: {out}");
        assert_eq!(
            out["detail"]["cost_usd"], "0.0000",
            "{args:?}: no model was asked: {out}"
        );
    }
    assert!(w.lines("schema-calls.log").is_empty());
}

/// Acceptance 5: `quality`'s masking case (`tests/quality.rs`, a rare name a fact and its
/// evidence both hold), run through `schema`: the name the run's model was shown as a
/// placeholder reaches the proposing model as a placeholder too.
#[test]
fn a_name_the_run_masks_reaches_the_proposing_model_masked() {
    let (w, name) = rare_instance("rare");
    // The model proposes a property of the person, naming the person in its reason by the
    // placeholder it was shown; the record holds the name restored.
    answer(
        &w,
        json!([{
            "change": {"kind": "add_property", "value": {"owner": "Person",
                "property": {"name": "kept_since", "value_kind": "Timestamp", "cardinality": "One"}}},
            "reason": "[Name-1] keeps the engine",
            "facts": ["FACT0"],
            "evidence": ["E1"],
        }]),
    );
    let (code, out) = w.cortex(&["schema", "rare", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let prompt = std::fs::read_to_string(w.root.join("schema-prompt-0.txt")).unwrap();
    assert!(
        prompt.contains("The engine is kept by"),
        "the model is shown the evidence: {prompt}"
    );
    assert!(
        !prompt.contains(&name),
        "a name the run's model was shown as a placeholder reaches the proposing model in the clear:\n{prompt}"
    );
    assert!(prompt.contains("[Name-"), "{prompt}");
    let lines = lines(&schema_dirs(&w, "rare")[0]);
    assert_eq!(
        lines[0]["reason"],
        format!("{name} keeps the engine"),
        "{lines:?}"
    );
}

/// Correction F3, the class: a placeholder in any name a change gives (a new type, a property,
/// an owner, an edge type's end, a merge's result) makes the proposal `invalid`, kept as the
/// model wrote it; the masked value enters neither the ontology nor `proposals.jsonl`.
#[test]
fn a_placeholder_in_any_name_a_change_gives_makes_it_invalid() {
    let (w, name) = rare_instance("named");
    let before = w.head("named");
    let with = |change: Value| json!({"change": change, "reason": "r", "facts": ["FACT0"], "evidence": []});
    answer(
        &w,
        json!([
            with(json!({"kind": "add_node_type", "value": {"name": "[Name-1]", "properties": []}})),
            with(
                json!({"kind": "add_node_type", "value": {"name": "Keeper", "properties": [
                {"name": "of_[Name-1]", "value_kind": "String", "cardinality": "One"}]}})
            ),
            with(json!({"kind": "add_property", "value": {"owner": "Person",
                "property": {"name": "[Name-1]", "value_kind": "String", "cardinality": "One"}}})),
            with(
                json!({"kind": "redeclare_property", "value": {"owner": "[Name-1]",
                "property": {"name": "role", "value_kind": "String", "cardinality": "Many"}}})
            ),
            with(json!({"kind": "add_edge_type", "value": {"name": "KNOWS",
                "source_types": ["Person"], "target_types": ["[Name-1]"]}})),
            with(
                json!({"kind": "merge_types", "value": {"types": ["Person"], "into": "[Name-1]"}})
            ),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "named", "--sample", "20"]);
    assert_eq!(
        (
            code,
            out["outcome"].as_str(),
            out["detail"]["invalid"].clone()
        ),
        (0, Some("applied"), json!(6)),
        "{out}"
    );
    assert_eq!(w.head("named"), before, "nothing was committed");
    let dir = &schema_dirs(&w, "named")[0];
    let written = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();
    assert!(!written.contains(&name), "{written}");
    for line in lines(dir) {
        assert_eq!(line["status"], "invalid", "{line}");
        assert!(
            line["note"]
                .as_str()
                .is_some_and(|n| n.contains("placeholder")),
            "{line}"
        );
    }
}

/// An instance whose store holds one person a run's model was shown as a placeholder (a rare
/// name, `rare_limit: 1`), with the proposer in place of the extraction stand-in; and the name.
fn rare_instance(instance: &str) -> (World, String) {
    let name = format!("{}{}", "Kowal", "czyk");
    let pages = format!(
        r#"{{"results":[{{"url":"https://example.org/a","title":"a","description":"about the engine.","content":"The engine is kept by {name} today.","content_truncated":false,"published":null,"score":"0.9"}}],"complete":true,"truncation":[],"provenance":{{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}}}"#
    );
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), common::answer(&pages)).unwrap();
    let path = w.spec(instance, "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n"),
    )
    .unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"prompt=$(cat)
printf '%s\n' "$prompt" > "{root}/run-prompt.txt"
id=$(printf '%s\n' "$prompt" | grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Person","parents":[],"abstract_type":false,"properties":[{{"name":"role","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[{{"node_type":"Person","aliases":["{name}"]}}],"facts":[{{"!Property":{{"subject":{{"node_type":"Person","aliases":["{name}"]}},"property":"role","value":{{"value_kind":"String","value":"keeper"}},"evidence":["$id"]}}}}]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{instance}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let run_prompt = std::fs::read_to_string(w.root.join("run-prompt.txt")).unwrap();
    assert!(
        !run_prompt.contains(&name) && run_prompt.contains("[Name-"),
        "the run's model is shown the name masked: {run_prompt}"
    );

    proposer(&w);
    (w, name)
}

#[test]
fn the_model_runs_isolated_with_its_own_prompt_and_sees_the_ontology_facts_and_evidence() {
    let w = instance("isolated", 3);
    answer(&w, json!([]));
    let (code, out) = w.cortex(&["schema", "isolated", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"]["proposed"], 0, "{out}");
    let args = w.lines("schema-args.log").join("\n");
    for flag in [
        "--tools  ",
        "--setting-sources ",
        "--strict-mcp-config",
        "--json-schema",
        "--system-prompt",
    ] {
        assert!(args.contains(flag), "{flag} missing: {args}");
    }
    assert!(args.contains("propose"), "its own system prompt: {args}");
    assert!(!args.contains("ekr.extraction-document/1"), "{args}");
    assert_eq!(
        w.lines("schema-env.log"),
        ["0"],
        "ANTHROPIC_API_KEY reached the model"
    );
    let prompt = std::fs::read_to_string(w.root.join("schema-prompt-0.txt")).unwrap();
    assert!(prompt.contains("Product"), "the ontology: {prompt}");
    assert!(prompt.contains("version"), "the ontology: {prompt}");
    assert!(prompt.contains("=== Evidence E1 ==="), "{prompt}");
    assert!(prompt.contains("=== Fact "), "{prompt}");
    let dirs = schema_dirs(&w, "isolated");
    assert_eq!(lines(&dirs[0]).len(), 0, "an empty answer is an empty file");
}

#[test]
fn a_change_cortex_cannot_write_against_the_ontology_is_invalid_and_not_applied() {
    let w = instance("invalid", 3);
    let before = w.head("invalid");
    answer(
        &w,
        json!([
            add_property("Nobody", "founded", "Integer", "FACT0", &[]),
            add_property("Product", "version", "String", "FACT0", &[]),
            {"change": {"kind": "add_node_type", "value": {"name": "Product", "properties": []}},
             "reason": "r", "facts": ["FACT0"], "evidence": []},
            {"change": {"kind": "add_edge_type", "value": {"name": "MADE_BY",
                "source_types": ["Product"], "target_types": ["Maker"]}},
             "reason": "r", "facts": ["FACT0"], "evidence": []},
            {"change": {"kind": "redeclare_property", "value": {"owner": "Product",
                "property": {"name": "colour", "value_kind": "String", "cardinality": "Many"}}},
             "reason": "r", "facts": ["FACT0"], "evidence": []},
        ]),
    );
    let (code, out) = w.cortex(&["schema", "invalid", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(
        (
            out["detail"]["invalid"].clone(),
            out["detail"]["applied"].clone()
        ),
        (json!(5), json!(0)),
        "{out}"
    );
    for line in lines(&schema_dirs(&w, "invalid")[0]) {
        assert_eq!(line["status"], "invalid", "{line}");
        assert!(
            line["note"].as_str().is_some_and(|n| !n.is_empty()),
            "{line}"
        );
    }
    assert_eq!(w.head("invalid"), before);
}

#[test]
fn new_types_and_a_redeclared_property_are_each_their_own_schema_version() {
    let w = instance("types", 3);
    let before = w.head("types");
    answer(
        &w,
        json!([
            {"change": {"kind": "add_node_type", "value": {"name": "Maker", "properties": [
                {"name": "country", "value_kind": "String", "cardinality": "One"}]}},
             "reason": "r", "facts": ["FACT0"], "evidence": ["E1"]},
            {"change": {"kind": "add_edge_type", "value": {"name": "MADE_BY",
                "source_types": ["Product"], "target_types": ["Maker"]}},
             "reason": "r", "facts": ["FACT0"], "evidence": ["E1"]},
            {"change": {"kind": "redeclare_property", "value": {"owner": "Product",
                "property": {"name": "version", "value_kind": "String", "cardinality": "Many"}}},
             "reason": "r", "facts": ["FACT0"], "evidence": ["E1"]},
        ]),
    );
    let (code, out) = w.cortex(&["schema", "types", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"]["applied"], 3, "{out}");
    let revisions: Vec<i64> = lines(&schema_dirs(&w, "types")[0])
        .iter()
        .filter_map(|l| l["revision"].as_i64())
        .collect();
    assert_eq!(revisions, [before + 1, before + 2, before + 3]);
    let now = ekr(&w, "types", &["ontology"]);
    let names = type_names(&now);
    assert!(names.contains(&"Maker".to_string()) && names.contains(&"MADE_BY".to_string()));
    assert_eq!(properties(&now, "Maker"), ["country"]);
    let version = now["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Product")
        .and_then(|t| t["properties"].as_array())
        .and_then(|p| p.iter().find(|p| p["name"] == "version"))
        .cloned()
        .unwrap();
    assert_eq!(version["cardinality"], "Many", "{version}");
}

/// The store holds `version` as text on every product; EKR refuses to redeclare it as an integer.
/// The refusal is recorded with EKR's codes, and the proposal after it still applies.
#[test]
fn a_change_ekr_refuses_is_recorded_with_its_codes_and_the_next_one_still_applies() {
    let w = instance("refusing", 3);
    let before = w.head("refusing");
    answer(
        &w,
        json!([
            {"change": {"kind": "redeclare_property", "value": {"owner": "Product",
                "property": {"name": "version", "value_kind": "Integer", "cardinality": "One"}}},
             "reason": "r", "facts": ["FACT0"], "evidence": []},
            add_property("Product", "founded", "Integer", "FACT0", &[]),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "refusing", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(
        (
            out["detail"]["refused"].clone(),
            out["detail"]["applied"].clone()
        ),
        (json!(1), json!(1)),
        "{out}"
    );
    let lines = lines(&schema_dirs(&w, "refusing")[0]);
    assert_eq!(lines[0]["status"], "refused", "{lines:?}");
    assert!(
        lines[0]["codes"].as_array().is_some_and(|c| !c.is_empty()),
        "{lines:?}"
    );
    assert!(lines[0].get("revision").is_none(), "{lines:?}");
    assert_eq!(lines[1]["status"], "applied", "{lines:?}");
    assert_eq!(lines[1]["revision"], before + 1, "{lines:?}");
}

#[test]
fn a_failed_model_call_applies_nothing_and_writes_no_proposals() {
    let w = instance("failing", 3);
    let before = w.head("failing");
    std::fs::write(w.root.join("schema-error"), "").unwrap();
    let (code, out) = w.cortex(&["schema", "failing", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("propose-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("cost 0.0200 USD"), "{out}");
    assert_eq!(w.head("failing"), before);
    for dir in schema_dirs(&w, "failing") {
        assert!(!dir.join("proposals.jsonl").exists(), "{}", dir.display());
    }
}

#[test]
fn a_sample_size_ekr_refuses_asks_no_model_and_writes_nothing() {
    let w = instance("refused", 3);
    let (code, out) = w.cortex(&["schema", "refused", "--sample", "0"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("sample-failed")),
        "{out}"
    );
    assert!(w.lines("schema-calls.log").is_empty());
    assert!(schema_dirs(&w, "refused").is_empty());
}

#[test]
fn an_unknown_instance_is_not_proposed_for() {
    let w = World::new();
    let (code, out) = w.cortex(&["schema", "nobody", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{out}"
    );
}

/// The sample is drawn under the home's lock and the model is asked without it, as `quality`
/// does: during the model's call another command takes the lock (`restore` answers `busy` while
/// it is held, `no-such-snapshot` once it is free).
#[test]
fn the_model_is_asked_without_the_home_lock_held() {
    let w = instance("unlocked", 3);
    answer(
        &w,
        json!([add_property(
            "Product",
            "founded",
            "Integer",
            "FACT0",
            &["E1"]
        )]),
    );
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-proposer")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
"{cortex}" restore unlocked no-such-snapshot > "$R/lock-probe.json" 2>&1
exec "$R/bin/claude-proposer" "$@"
"#,
            root = w.root.display(),
            cortex = env!("CARGO_BIN_EXE_cortex"),
        ),
    );
    let (code, out) = w.cortex(&["schema", "unlocked", "--sample", "3"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"]["applied"], 1, "{out}");
    let probe: Value =
        serde_json::from_str(&std::fs::read_to_string(w.root.join("lock-probe.json")).unwrap())
            .unwrap_or(Value::Null);
    assert_eq!(
        probe["outcome"], "no-such-snapshot",
        "another command found the home's lock held while the model was asked: {probe}"
    );
}

/// `cortex schema` without an instance prints the spec file's JSON Schema, as before.
#[test]
fn schema_without_an_instance_still_prints_the_spec_files_json_schema() {
    let w = World::new();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("schema")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success());
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        doc["$defs"].get("cortex.instance.InstanceSpec").is_some(),
        "{doc}"
    );
}

/// Correction F1 and the case note, the class: after a change, no type may hold two properties
/// of one name (its own and those it inherits), and no two types may share a name, in any case.
/// A proposal that would break either is `invalid`, from either side of the inheritance; one that
/// breaks neither still applies.
#[test]
fn a_name_a_type_would_hold_twice_is_invalid_in_any_case_and_from_either_side() {
    let w = instance("twice", 3);
    adv_maker_and_startup(&w, "twice");
    let path = w.root.join("startup-founded.json");
    std::fs::write(
        &path,
        json!({"format": "ekr.extraction-document/1",
            "ontology": {"node_types": [
                {"name": "Maker", "parents": [], "abstract_type": false, "properties": [
                    {"name": "country", "value": {"value_kind": "String"},
                     "cardinality": "One", "required": true}]},
                {"name": "Startup", "parents": ["Maker"], "abstract_type": false,
                 "properties": [{"name": "founded", "value": {"value_kind": "Integer"},
                     "cardinality": "One", "required": false}]}],
              "edge_types": []},
            "entities": [], "facts": [], "evidence": []})
        .to_string(),
    )
    .unwrap();
    let report = ekr(&w, "twice", &["apply-extraction", path.to_str().unwrap()]);
    assert_eq!(report["rejected"], json!([]), "{report}");
    let now = ekr(&w, "twice", &["ontology"]);
    assert_eq!(properties(&now, "Startup"), ["founded"], "{now}");
    let before = w.head("twice");
    let with = |change: Value| json!({"change": change, "reason": "r", "facts": ["FACT0"], "evidence": []});
    let prop = |name: &str| json!({"name": name, "value_kind": "String", "cardinality": "One"});
    answer(
        &w,
        json!([
            with(json!({"kind": "add_node_type", "value": {"name": "product", "properties": []}})),
            with(json!({"kind": "add_edge_type", "value": {"name": "MAKER",
                "source_types": ["Product"], "target_types": ["Maker"]}})),
            with(
                json!({"kind": "add_property", "value": {"owner": "Product", "property": prop("Version")}})
            ),
            with(
                json!({"kind": "add_property", "value": {"owner": "Maker", "property": prop("founded")}})
            ),
            with(
                json!({"kind": "add_property", "value": {"owner": "Startup", "property": prop("COUNTRY")}})
            ),
            with(
                json!({"kind": "redeclare_property", "value": {"owner": "Startup", "property": prop("country")}})
            ),
            with(json!({"kind": "add_node_type", "value": {"name": "Keeper",
                "properties": [prop("since"), prop("Since")]}})),
            with(
                json!({"kind": "add_property", "value": {"owner": "Maker", "property": prop("website")}})
            ),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "twice", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let lines = lines(&schema_dirs(&w, "twice")[0]);
    let statuses: Vec<&str> = lines.iter().filter_map(|l| l["status"].as_str()).collect();
    assert_eq!(
        statuses,
        ["invalid", "invalid", "invalid", "invalid", "invalid", "invalid", "invalid", "applied"],
        "{lines:?}"
    );
    assert_eq!(w.head("twice"), before + 1);
    let now = ekr(&w, "twice", &["ontology"]);
    assert_eq!(properties(&now, "Maker"), ["country", "website"], "{now}");
    assert_eq!(properties(&now, "Startup"), ["founded"], "{now}");
    let names = type_names(&now);
    assert_eq!(
        names
            .iter()
            .filter(|n| n.eq_ignore_ascii_case("product"))
            .count(),
        1
    );
    assert_eq!(
        names
            .iter()
            .filter(|n| n.eq_ignore_ascii_case("maker"))
            .count(),
        1
    );
}

// Adversary cases (wave 20261006h, unit a, pass 1).

/// `ekr apply-extraction` of an ontology-only document over the instance's store, as a run's
/// extraction or the spec's `seed.schema` commits one: `Maker` with a required `country`, and
/// `Startup`, a subtype of `Maker`.
fn adv_maker_and_startup(w: &World, name: &str) {
    let path = w.root.join("adv-ontology.json");
    std::fs::write(
        &path,
        json!({"format": "ekr.extraction-document/1",
            "ontology": {"node_types": [
                {"name": "Maker", "parents": [], "abstract_type": false, "properties": [
                    {"name": "country", "value": {"value_kind": "String"},
                     "cardinality": "One", "required": true}]},
                {"name": "Startup", "parents": ["Maker"], "abstract_type": false,
                 "properties": []}],
              "edge_types": []},
            "entities": [], "facts": [], "evidence": []})
        .to_string(),
    )
    .unwrap();
    let report = ekr(w, name, &["apply-extraction", path.to_str().unwrap()]);
    assert_eq!(report["rejected"], json!([]), "{report}");
    assert_eq!(
        report["committed"].as_array().map(Vec::len),
        Some(1),
        "{report}"
    );
}

fn adv_property(ontology: &Value, type_name: &str, property: &str) -> Value {
    ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["name"] == type_name)
        .and_then(|t| t["properties"].as_array())
        .and_then(|p| p.iter().find(|p| p["name"] == property))
        .cloned()
        .unwrap_or(Value::Null)
}

/// A subtype inherits `country` from `Maker`; adding `country` to it again gives it a second
/// property of that name, and EKR then refuses every `Startup.country` fact a run extracts as the
/// inherited text (`extraction-value-mismatch`). The property already is there, so the change is
/// not applied.
#[test]
fn adv_a_property_a_subtype_inherits_is_not_added_to_it_again() {
    let w = instance("inherits", 3);
    adv_maker_and_startup(&w, "inherits");
    let before = w.head("inherits");
    answer(
        &w,
        json!([add_property(
            "Startup",
            "country",
            "Integer",
            "FACT0",
            &["E1"]
        )]),
    );
    let (code, out) = w.cortex(&["schema", "inherits", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let now = ekr(&w, "inherits", &["ontology"]);
    assert!(
        !properties(&now, "Startup").contains(&"country".to_string()),
        "Startup now declares a second `country` beside the one it inherits from Maker: {:?}",
        now["node_types"]
    );
    let lines = lines(&schema_dirs(&w, "inherits")[0]);
    assert_ne!(lines[0]["status"], "applied", "{lines:?}");
    assert_eq!(w.head("inherits"), before, "nothing was committed");
}

/// A redeclaration changes what it names, the value kind and the cardinality. The model is not
/// shown whether a property is required, and the proposal cannot say; a property the store
/// declares required stays required.
#[test]
fn adv_redeclaring_a_property_keeps_it_required() {
    let w = instance("required", 3);
    adv_maker_and_startup(&w, "required");
    let then = adv_property(&ekr(&w, "required", &["ontology"]), "Maker", "country");
    assert_eq!(then["required"], true, "{then}");
    answer(
        &w,
        json!([{"change": {"kind": "redeclare_property", "value": {"owner": "Maker",
            "property": {"name": "country", "value_kind": "String", "cardinality": "Many"}}},
            "reason": "a maker is in several countries", "facts": ["FACT0"], "evidence": ["E1"]}]),
    );
    let (code, out) = w.cortex(&["schema", "required", "--sample", "5"]);
    assert_eq!(
        (
            code,
            out["outcome"].as_str(),
            out["detail"]["applied"].clone()
        ),
        (0, Some("applied"), json!(1)),
        "{out}"
    );
    let now = adv_property(&ekr(&w, "required", &["ontology"]), "Maker", "country");
    assert_eq!(now["cardinality"], "Many", "the change asked for: {now}");
    assert_eq!(
        now["required"], true,
        "the redeclaration also made `country` optional, which nobody proposed: {then} -> {now}"
    );
}

/// Acceptance 5 over two rounds: the model, shown a rare name as `[Name-1]`, names a property
/// with the placeholder; restored, the name becomes a property of the ontology, and the next
/// round shows the ontology's names as they are.
#[test]
fn adv_a_masked_name_does_not_reach_the_next_round_through_a_property_name() {
    let name = format!("{}{}", "Kowal", "czyk");
    let pages = format!(
        r#"{{"results":[{{"url":"https://example.org/a","title":"a","description":"about the engine.","content":"The engine is kept by {name} today.","content_truncated":false,"published":null,"score":"0.9"}}],"complete":true,"truncation":[],"provenance":{{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}}}"#
    );
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), common::answer(&pages)).unwrap();
    let path = w.spec("rare2", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n"),
    )
    .unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"prompt=$(cat)
id=$(printf '%s\n' "$prompt" | grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Person","parents":[],"abstract_type":false,"properties":[{{"name":"role","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[{{"node_type":"Person","aliases":["{name}"]}}],"facts":[{{"!Property":{{"subject":{{"node_type":"Person","aliases":["{name}"]}},"property":"role","value":{{"value_kind":"String","value":"keeper"}},"evidence":["$id"]}}}}]}}}}
EOF
"#
        ),
    );
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "rare2/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");

    proposer(&w);
    answer(
        &w,
        json!([{
            "change": {"kind": "add_property", "value": {"owner": "Person",
                "property": {"name": "[Name-1]", "value_kind": "String", "cardinality": "One"}}},
            "reason": "the keeper",
            "facts": ["FACT0"],
            "evidence": ["E1"],
        }]),
    );
    let (code, out) = w.cortex(&["schema", "rare2", "--sample", "20"]);
    assert_eq!(code, 0, "{out}");
    let first = std::fs::read_to_string(w.root.join("schema-prompt-0.txt")).unwrap();
    assert!(!first.contains(&name), "{first}");

    answer(&w, json!([]));
    let (code, out) = w.cortex(&["schema", "rare2", "--sample", "20"]);
    assert_eq!(code, 0, "{out}");
    let second = std::fs::read_to_string(w.root.join("schema-prompt-1.txt")).unwrap();
    assert!(
        !second.contains(&name),
        "a name the run masks reaches the proposing model in the clear in the next round:\n{second}"
    );
}

/// A store holding only the seed: no fact to draw, so no model is asked and nothing is applied.
#[test]
fn adv_an_empty_store_asks_no_model_and_applies_nothing() {
    let w = World::new();
    let path = w.spec("empty", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    proposer(&w);
    answer(
        &w,
        json!([add_property("Nobody", "x", "String", "FACT0", &[])]),
    );
    let before = w.head("empty");
    let (code, out) = w.cortex(&["schema", "empty", "--sample", "5"]);
    assert!(
        matches!(
            (code, out["outcome"].as_str()),
            (0, Some("applied")) | (1, Some("sample-failed"))
        ),
        "{out}"
    );
    if code == 0 {
        assert_eq!(out["detail"]["proposed"], 0, "{out}");
        assert_eq!(
            out["detail"]["cost_usd"], "0.0000",
            "no model was asked: {out}"
        );
    }
    assert!(w.lines("schema-calls.log").is_empty(), "{out}");
    assert_eq!(w.head("empty"), before);
}

/// A run commits `Maker` while the model is asked (the lock is free then). EKR accepts a second
/// node type of one name, and every later `apply-extraction` then refuses the whole document
/// ("two node types are named"), so the proposal of `Maker` is decided against the ontology at
/// the head when it is applied, not the one the sample was drawn with.
#[test]
fn adv_a_type_a_run_adds_while_the_model_is_asked_is_not_defined_twice() {
    let w = instance("raced", 3);
    let dir = w.home.join("instances").join("raced");
    std::fs::write(
        w.root.join("maker.json"),
        json!({"format": "ekr.extraction-document/1",
            "ontology": {"node_types": [{"name": "Maker", "parents": [], "abstract_type": false,
                "properties": []}], "edge_types": []},
            "entities": [], "facts": [], "evidence": []})
        .to_string(),
    )
    .unwrap();
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-proposer")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
EKR_HOST="{dir}/host.json" EKR_BACKEND=sqlite EKR_STORE="{dir}/store.sqlite" "{ekr}" apply-extraction "$R/maker.json" > "$R/race.json" 2>&1 < /dev/null
exec "$R/bin/claude-proposer" "$@"
"#,
            root = w.root.display(),
            dir = dir.display(),
            ekr = common::ekr().display(),
        ),
    );
    answer(
        &w,
        json!([{"change": {"kind": "add_node_type", "value": {"name": "Maker", "properties": []}},
            "reason": "r", "facts": ["FACT0"], "evidence": ["E1"]}]),
    );
    let (code, out) = w.cortex(&["schema", "raced", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    let race = std::fs::read_to_string(w.root.join("race.json")).unwrap();
    assert!(
        race.contains("\"committed\""),
        "the run did not commit: {race}"
    );
    let now = ekr(&w, "raced", &["ontology"]);
    let makers = type_names(&now).iter().filter(|n| *n == "Maker").count();
    assert_eq!(makers, 1, "{now}");
    let lines = lines(&schema_dirs(&w, "raced")[0]);
    assert_eq!(lines[0]["status"], "invalid", "{lines:?}");
}

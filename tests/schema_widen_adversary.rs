//! Adversary cases for `story:schema-widens-edge-type`: the wire form and the answer schema of
//! `widen_edge_type`, what the model is shown, ends written whole (no narrowing, no duplicates,
//! held order first), names resolved exactly and by kind, a widening beside an `add_node_type`,
//! an `add_edge_type` or a `remove_edge_type` of the same round, a placeholder of the redaction
//! policy in a name, and EKR refusing a widening, on the review path and without `--review`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// The proposing model's stand-in: logs its arguments to `schema-args.log` and its prompt to
/// `schema-prompt.txt`, and answers `proposals.json` with `FACT0` the first fact id of its prompt.
fn proposer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
printf "%s\n" "$*" | tr "\n" " " >> "$R/schema-args.log"; echo >> "$R/schema-args.log"
prompt=$(cat)
printf '%s\n' "$prompt" > "$R/schema-prompt.txt"
fact=$(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
proposals=$(sed -e "s/FACT0/$fact/g" "$R/proposals.json")
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.02,\"structured_output\":{{\"proposals\":$proposals}}}}"
"#,
            root = w.root.display()
        ),
    );
}

fn answer(w: &World, proposals: Value) {
    std::fs::write(w.root.join("proposals.json"), proposals.to_string()).unwrap();
}

/// An `ekr` in front of the real one: `ekr ontology` answers `ontology.json` while that file
/// exists, every other call is the real `ekr`'s.
fn ekr_front(w: &World) -> PathBuf {
    let path = w.bin.join("ekr");
    executable(
        &path,
        &format!(
            r#"if [ "$1" = ontology ] && [ -e "{root}/ontology.json" ]; then cat "{root}/ontology.json"; exit 0; fi
exec "{real}" "$@"
"#,
            root = w.root.display(),
            real = common::ekr().display()
        ),
    );
    path
}

/// An instance `name` over the world's default extraction (`Organization`, `Product`,
/// `DEVELOPS` from `Organization` to `Product`) after one run, with the proposer in place of the
/// extraction stand-in. With `front`, its spec names [`ekr_front`] as its `ekr`.
fn instance_with(name: &str, front: bool) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    if front {
        let front = ekr_front(&w);
        let text = std::fs::read_to_string(&path).unwrap().replace(
            &format!("bin: \"{}\"", common::ekr().display()),
            &format!("bin: \"{}\"", front.display()),
        );
        assert!(text.contains(&front.display().to_string()), "{text}");
        std::fs::write(&path, text).unwrap();
    }
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    w
}

fn instance(name: &str) -> World {
    instance_with(name, false)
}

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

fn with(change: Value) -> Value {
    json!({"change": change, "reason": "the evidence calls for it", "facts": ["FACT0"],
           "evidence": ["E1"]})
}

fn widen(edge: &str, sources: &[&str], targets: &[&str]) -> Value {
    with(json!({"kind": "widen_edge_type", "value": {"name": edge,
        "add_source_types": sources, "add_target_types": targets}}))
}

fn node_type(name: &str) -> Value {
    with(json!({"kind": "add_node_type", "value": {"name": name, "properties": []}}))
}

fn edge_type(name: &str, sources: &[&str], targets: &[&str]) -> Value {
    with(json!({"kind": "add_edge_type", "value": {"name": name,
        "source_types": sources, "target_types": targets}}))
}

/// The only run directory under `schema/` of `name`.
fn only_run(w: &World, name: &str) -> PathBuf {
    let dirs: Vec<PathBuf> = std::fs::read_dir(w.home.join("instances").join(name).join("schema"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs[0].clone()
}

fn lines(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("proposals.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn statuses(dir: &Path) -> Vec<String> {
    lines(dir)
        .iter()
        .map(|l| l["status"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The names of one end of the edge type `edge`, as `ekr ontology` lists them, sorted.
fn ends(ontology: &Value, edge: &str, end: &str) -> Vec<String> {
    let t = ontology["edge_types"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["name"] == edge)
        .unwrap_or_else(|| panic!("no edge type {edge}: {ontology}"));
    let mut names: Vec<String> = t[end]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["name"].as_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

fn type_id(ontology: &Value, kind: &str, name: &str) -> String {
    ontology[kind]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .and_then(|t| t["id"].as_str())
        .unwrap_or_else(|| panic!("no {name} in {ontology}"))
        .to_string()
}

/// Each `!WidenEdgeType` written to a `schema-<n>.yaml` of `dir`, in file order: its source and
/// target ids, as written.
fn written_widenings(dir: &Path) -> Vec<(Vec<String>, Vec<String>)> {
    fn walk(v: &serde_yaml_ng::Value, out: &mut Vec<(Vec<String>, Vec<String>)>) {
        use serde_yaml_ng::Value as Y;
        match v {
            Y::Tagged(t) => {
                if t.tag == "WidenEdgeType" {
                    let ids = |key: &str| -> Vec<String> {
                        t.value[key]
                            .as_sequence()
                            .into_iter()
                            .flatten()
                            .filter_map(|i| i.as_str().map(str::to_string))
                            .collect()
                    };
                    out.push((ids("source_types"), ids("target_types")));
                }
                walk(&t.value, out);
            }
            Y::Sequence(s) => s.iter().for_each(|v| walk(v, out)),
            Y::Mapping(m) => m.values().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("schema-") && n.ends_with(".yaml"))
        })
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        walk(&doc, &mut out);
    }
    out
}

/// The wire form reads back; a malformed widening reads as nothing; the answer schema the model is
/// held to and its system prompt both carry the variant with its three fields.
#[test]
fn the_wire_form_of_a_widening_reads_back_and_the_answer_schema_carries_it() {
    use cortex_cli::schema::{answer_schema, change_json, change_of, SYSTEM_PROMPT};
    let wire = json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
        "add_source_types": ["Vendor", "Person"], "add_target_types": []}});
    let change = change_of(&wire).expect("a widening reads");
    assert_eq!(change_json(&change), wire);
    for malformed in [
        json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
            "add_source_types": "Vendor", "add_target_types": []}}),
        json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
            "add_source_types": ["Vendor"]}}),
        json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
            "add_source_types": [1], "add_target_types": []}}),
        json!({"kind": "widen_edge_type", "value": {
            "add_source_types": ["Vendor"], "add_target_types": []}}),
        json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
            "source_types": ["Vendor"], "target_types": []}}),
    ] {
        assert_eq!(change_of(&malformed), None, "{malformed}");
    }

    let schema = answer_schema();
    let variants = schema["$defs"]["cortex.instance.SchemaChange"]["oneOf"]
        .as_array()
        .unwrap();
    let widen = variants
        .iter()
        .find(|v| v["properties"]["kind"]["const"] == "widen_edge_type")
        .unwrap_or_else(|| panic!("no widen_edge_type in {schema}"));
    let value = widen["properties"]["value"].to_string();
    let def = schema["$defs"]["cortex.instance.EdgeTypeWidening"].to_string();
    for field in ["name", "add_source_types", "add_target_types"] {
        assert!(
            value.contains(field) || def.contains(field),
            "{field} in {value} / {def}"
        );
    }
    assert!(SYSTEM_PROMPT.contains("`widen_edge_type`"), "{SYSTEM_PROMPT}");
}

/// The model is asked with the variant in its schema and its instruction, and is shown each edge
/// type with its ends by name; a malformed widening in its answer is dropped and the rest kept.
#[test]
fn the_model_is_shown_the_widening_and_a_malformed_one_is_dropped() {
    let w = instance("wp");
    answer(
        &w,
        json!([
            with(json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
                "add_source_types": "Product", "add_target_types": []}})),
            with(json!({"kind": "widen_edge_type", "value": {"name": "DEVELOPS",
                "add_source_types": ["Product"]}})),
            widen("DEVELOPS", &["Product"], &[]),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "wp", "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (&d["proposed"], &d["dropped"], &d["pending"]),
        (&json!(1), &json!(2), &json!(1)),
        "{out}"
    );
    let args = w.lines("schema-args.log").join("\n");
    assert!(args.contains("widen_edge_type"), "{args}");
    assert!(args.contains("add_source_types"), "{args}");
    let prompt = std::fs::read_to_string(w.root.join("schema-prompt.txt")).unwrap();
    assert!(
        prompt.contains("- DEVELOPS: Organization -> Product"),
        "the edge type is shown with its ends: {prompt}"
    );
}

/// Without `--review`: a node type the round defined first is resolved, the source end is written
/// whole with what it held first, a type named twice or already held is written once, and the
/// target end that does not grow is written as it is.
#[test]
fn a_round_without_review_widens_whole_with_a_node_type_it_defined_first() {
    let w = instance("wn");
    answer(
        &w,
        json!([
            node_type("Vendor"),
            widen("DEVELOPS", &["Vendor", "Vendor", "Organization"], &["Product"]),
        ]),
    );
    let head = w.head("wn");
    let (code, out) = w.cortex(&["schema", "wn", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    let d = &out["detail"];
    assert_eq!(
        (&d["applied"], &d["invalid"], &d["refused"]),
        (&json!(2), &json!(0), &json!(0)),
        "{out}"
    );
    assert_eq!(w.head("wn"), head + 2, "two schema transactions");
    let ontology = ekr(&w, "wn", &["ontology"]);
    assert_eq!(
        ends(&ontology, "DEVELOPS", "source_types"),
        ["Organization", "Vendor"]
    );
    assert_eq!(ends(&ontology, "DEVELOPS", "target_types"), ["Product"]);
    let (org, vendor, product) = (
        type_id(&ontology, "node_types", "Organization"),
        type_id(&ontology, "node_types", "Vendor"),
        type_id(&ontology, "node_types", "Product"),
    );
    assert_eq!(
        written_widenings(&only_run(&w, "wn")),
        [(vec![org, vendor], vec![product])],
        "each end whole, the held type first, each type once"
    );
}

/// A widening of an edge type an `add_edge_type` of the same round defined plans against it, both
/// without `--review` and on the review path.
#[test]
fn a_widening_after_an_add_edge_type_of_the_same_round_widens_the_new_type() {
    let proposals = json!([
        edge_type("SUPPLIES", &["Organization"], &["Product"]),
        widen("SUPPLIES", &[], &["Organization"]),
    ]);
    // Without review.
    let w = instance("we");
    answer(&w, proposals.clone());
    let (code, out) = w.cortex(&["schema", "we", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        (&out["detail"]["applied"], &out["detail"]["invalid"]),
        (&json!(2), &json!(0)),
        "{out}"
    );
    let ontology = ekr(&w, "we", &["ontology"]);
    assert_eq!(
        ends(&ontology, "SUPPLIES", "source_types"),
        ["Organization"]
    );
    assert_eq!(
        ends(&ontology, "SUPPLIES", "target_types"),
        ["Organization", "Product"]
    );

    // Review, approve both, apply.
    let w = instance("wr");
    answer(&w, proposals);
    let (code, out) = w.cortex(&["schema", "wr", "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    let stamp = out["detail"]["stamp"].as_str().unwrap().to_string();
    let (code, out) = w.cortex(&["schema", "approve", "wr", &stamp]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = w.cortex(&["schema", "apply", "wr", &stamp]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["detail"]["applied"], 2, "{out}");
    let ontology = ekr(&w, "wr", &["ontology"]);
    assert_eq!(
        ends(&ontology, "SUPPLIES", "target_types"),
        ["Organization", "Product"]
    );
}

/// `remove_edge_type` is recorded only, so a widening of the same edge type in the same round is
/// still planned against the edge type the ontology holds, and applied.
#[test]
fn a_widening_of_an_edge_type_removed_in_the_same_round_is_applied() {
    let w = instance("wx");
    answer(
        &w,
        json!([
            with(json!({"kind": "remove_edge_type", "value": {"name": "DEVELOPS"}})),
            widen("DEVELOPS", &["Product"], &[]),
        ]),
    );
    let (code, out) = w.cortex(&["schema", "wx", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(statuses(&only_run(&w, "wx")), ["recorded-only", "applied"]);
    let ontology = ekr(&w, "wx", &["ontology"]);
    assert_eq!(
        ends(&ontology, "DEVELOPS", "source_types"),
        ["Organization", "Product"]
    );
}

/// Names resolve exactly, as every other change names an existing type: an edge type or a node
/// type named in another case, and an edge type's name given as a node type, are each `invalid`
/// with a note naming it, and nothing is submitted.
#[test]
fn a_widening_resolves_names_exactly_and_node_types_only_among_node_types() {
    let w = instance("wc");
    answer(
        &w,
        json!([
            widen("develops", &["Product"], &[]),
            widen("DEVELOPS", &["product"], &[]),
            widen("DEVELOPS", &["DEVELOPS"], &[]),
        ]),
    );
    let head = w.head("wc");
    let (code, out) = w.cortex(&["schema", "wc", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["detail"]["invalid"], 3, "{out}");
    assert_eq!(w.head("wc"), head, "nothing was submitted");
    let dir = only_run(&w, "wc");
    for (line, named) in lines(&dir).iter().zip(["develops", "product", "DEVELOPS"]) {
        assert_eq!(line["status"], "invalid", "{line}");
        assert!(
            line["note"].as_str().is_some_and(|n| n.contains(named)),
            "{named} in {line}"
        );
    }
}

/// A placeholder of the redaction policy in a type a widening names makes it `invalid` with the
/// placeholder note, before any plan, and the masked name enters nothing.
#[test]
fn a_placeholder_in_a_widening_makes_it_invalid() {
    let name = format!("{}{}", "Kowal", "czyk");
    let pages = format!(
        r#"{{"results":[{{"url":"https://example.org/a","title":"a","description":"about the engine.","content":"Example Labs develops the Widget engine with {name} today.","content_truncated":false,"published":null,"score":"0.9"}}],"complete":true,"truncation":[],"provenance":{{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}}}"#
    );
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), common::answer(&pages)).unwrap();
    let path = w.spec("wm", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n"),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "wm/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    answer(
        &w,
        json!([
            widen("DEVELOPS", &["[Name-1]"], &[]),
            widen("DEVELOPS", &[], &["[Name-1]"]),
        ]),
    );
    let head = w.head("wm");
    let (code, out) = w.cortex(&["schema", "wm", "--sample", "20"]);
    assert_eq!(code, 0, "{out}");
    let prompt = std::fs::read_to_string(w.root.join("schema-prompt.txt")).unwrap();
    assert!(
        !prompt.contains(&name) && prompt.contains("[Name-1]"),
        "the model is shown the name masked: {prompt}"
    );
    assert_eq!(out["detail"]["invalid"], 2, "{out}");
    assert_eq!(w.head("wm"), head);
    let dir = only_run(&w, "wm");
    let written = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();
    assert!(!written.contains(&name), "{written}");
    for line in lines(&dir) {
        assert!(
            line["note"]
                .as_str()
                .is_some_and(|n| n.contains("placeholder")),
            "{line}"
        );
    }
}

/// EKR refusing a widening: cortex is shown an ontology whose `DEVELOPS` sources are empty (a
/// stale read, built here by an `ekr` in front of the real one), so the end it writes drops
/// `Organization`; EKR refuses it with `edge-endpoint-removed`. The proposal is `refused` with
/// EKR's code in `codes`, without `--review` and through approve and apply, and the store's
/// `DEVELOPS` keeps its ends.
#[test]
fn ekr_refusing_a_widening_records_it_refused_with_ekrs_code() {
    let w = instance_with("wf", true);
    let mut stale = ekr(&w, "wf", &["ontology"]);
    for t in stale["edge_types"].as_array_mut().unwrap() {
        if t["name"] == "DEVELOPS" {
            t["source_types"] = json!([]);
        }
    }
    std::fs::write(w.root.join("ontology.json"), stale.to_string()).unwrap();
    answer(&w, json!([widen("DEVELOPS", &["Product"], &[])]));

    let check = |w: &World, dir: &Path| {
        let l = &lines(dir)[0];
        assert_eq!(l["status"], "refused", "{l}");
        assert!(
            l["codes"]
                .as_array()
                .is_some_and(|c| c.iter().any(|c| c == "edge-endpoint-removed")),
            "EKR's code in codes: {l}"
        );
        let ontology = ekr(w, "wf", &["ontology"]);
        assert_eq!(ends(&ontology, "DEVELOPS", "source_types"), ["Organization"]);
    };

    let (code, out) = w.cortex(&["schema", "wf", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["detail"]["refused"], 1, "{out}");
    check(&w, &only_run(&w, "wf"));

    let (code, out) = w.cortex(&["schema", "wf", "--sample", "5", "--review"]);
    assert_eq!(code, 0, "{out}");
    let stamp = out["detail"]["stamp"].as_str().unwrap().to_string();
    let (code, out) = w.cortex(&["schema", "approve", "wf", &stamp]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = w.cortex(&["schema", "apply", "wf", &stamp]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        (&out["detail"]["refused"], &out["detail"]["applied"]),
        (&json!(1), &json!(0)),
        "{out}"
    );
    let dir = w
        .home
        .join("instances")
        .join("wf")
        .join("schema")
        .join(&stamp);
    check(&w, &dir);
}

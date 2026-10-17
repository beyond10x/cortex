//! The known entities a run shows the model before a batch (`cortex.instance.KnownEntity`): the
//! store's nodes, of any type, that one of the batch's keys names by their canonical name or an
//! alias (origin `Store`), ahead of the names the run's own batches extracted (origin `Run`, at
//! most [`RUN_PER_TYPE`] per node type). The cap holds for run names only, so every store hit of
//! the batch is shown. No noun is persisted: the store is read once per batch (`ekr snapshot`).

use std::collections::{BTreeMap, BTreeSet};

use cortex_model::instance as m;
use serde_json::Value;

use crate::evidence::Issued;
use crate::sources::Origin;

/// Known entity names per node type, of origin `Run`, given to the model.
pub const RUN_PER_TYPE: usize = 50;

/// How a name is compared: case folded.
pub fn fold(text: &str) -> String {
    text.to_lowercase()
}

/// The terms of `batch` a store node may be named by, each [`fold`]ed: each document's key and, for
/// a record, its id: after the last `#` of a file record's key, after `<adapter>:<operation>:` of
/// a Connectors record's.
pub fn terms(batch: &[Issued]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for issued in batch {
        let key = &issued.doc.key;
        out.insert(fold(key));
        let id = match issued.doc.origin {
            Origin::FileRecord => key.rsplit_once('#').map(|(_, id)| id),
            Origin::Record => key.splitn(3, ':').nth(2),
            Origin::Url | Origin::File => None,
        };
        if let Some(id) = id.filter(|id| !id.trim().is_empty()) {
            out.insert(fold(id));
        }
    }
    out
}

/// Every node of `snapshot` (`ekr snapshot`) whose canonical name or one of whose aliases, folded,
/// is one of `terms`, of any type, each once, as a known entity of origin `Store`: its type's name
/// from `ontology` (`ekr ontology`), its canonical name and its other aliases. In the snapshot's
/// node order; a node of a type the ontology does not name is left out.
pub fn store_hits(
    snapshot: &Value,
    ontology: &Value,
    terms: &BTreeSet<String>,
) -> Vec<m::KnownEntity> {
    if terms.is_empty() {
        return Vec::new();
    }
    let types: BTreeMap<&str, &str> = ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| Some((t["id"].as_str()?, t["name"].as_str()?)))
        .collect();
    let mut hits = Vec::new();
    for node in snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, n)| n)
    {
        let Some(name) = node["canonical_name"].as_str() else {
            continue;
        };
        let aliases: Vec<&str> = node["aliases"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let named = std::iter::once(name)
            .chain(aliases.iter().copied())
            .any(|n| terms.contains(&fold(n)));
        if !named {
            continue;
        }
        let Some(node_type) = node["type_id"].as_str().and_then(|id| types.get(id)) else {
            continue;
        };
        let mut others: Vec<String> = Vec::new();
        for alias in aliases {
            if alias != name && !others.iter().any(|o| o == alias) {
                others.push(alias.to_string());
            }
        }
        hits.push(m::KnownEntity {
            node_type: node_type.to_string(),
            name: name.to_string(),
            aliases: others,
            origin: m::KnownOrigin::Store,
        });
    }
    hits
}

/// `store` hits, then the run's own names (`run`, by node type), at most [`RUN_PER_TYPE`] of a
/// type, as known entities of origin `Run`. A run name equal, folded, to the name or an alias of a
/// store hit of its type is shown once, as the store hit.
pub fn merge(
    store: Vec<m::KnownEntity>,
    run: &BTreeMap<String, Vec<String>>,
) -> Vec<m::KnownEntity> {
    let held: BTreeSet<(String, String)> = store
        .iter()
        .flat_map(|e| {
            std::iter::once(&e.name)
                .chain(&e.aliases)
                .map(|n| (e.node_type.clone(), fold(n)))
        })
        .collect();
    let mut out = store;
    for (ty, names) in run {
        let kept = names
            .iter()
            .filter(|n| !held.contains(&(ty.clone(), fold(n))))
            .take(RUN_PER_TYPE)
            .map(|n| m::KnownEntity {
                node_type: ty.clone(),
                name: n.clone(),
                aliases: Vec::new(),
                origin: m::KnownOrigin::Run,
            });
        out.extend(kept);
    }
    out
}

/// The known entities of `batch`: [`store_hits`] of one read of `store` (`ekr snapshot`),
/// [`merge`]d with the run's own names `run`.
pub fn for_batch(
    store: &crate::ekr::Store,
    ontology: &Value,
    batch: &[Issued],
    run: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<m::KnownEntity>, String> {
    let terms = terms(batch);
    let hits = if terms.is_empty() {
        Vec::new()
    } else {
        store_hits(&store.snapshot()?, ontology, &terms)
    };
    Ok(merge(hits, run))
}

/// The known-entity lines of a prompt, one per node type in name order: `- <type>: ` and each
/// entity of it, store hits first, separated by `; `. A run name is written as it is; a store hit
/// as `<name> (in the store; also <alias>, <alias>)`.
pub fn lines(entities: &[m::KnownEntity]) -> Vec<String> {
    let mut by_type: BTreeMap<&str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for e in entities {
        let (store, run) = by_type.entry(e.node_type.as_str()).or_default();
        match e.origin {
            m::KnownOrigin::Store if e.aliases.is_empty() => {
                store.push(format!("{} (in the store)", e.name));
            }
            m::KnownOrigin::Store => store.push(format!(
                "{} (in the store; also {})",
                e.name,
                e.aliases.join(", ")
            )),
            m::KnownOrigin::Run => run.push(e.name.clone()),
        }
    }
    by_type
        .into_iter()
        .map(|(ty, (store, run))| format!("- {ty}: {}", [store, run].concat().join("; ")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Document;
    use serde_json::json;

    fn issued(key: &str, origin: Origin) -> Issued {
        Issued {
            id: "e".into(),
            item: serde_yaml_ng::Value::Null,
            doc: Document {
                key: key.into(),
                origin,
                title: None,
                description: None,
                published: None,
                text: String::new(),
                hash: None,
            },
        }
    }

    fn store(ty: &str, name: &str, aliases: &[&str]) -> m::KnownEntity {
        m::KnownEntity {
            node_type: ty.into(),
            name: name.into(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            origin: m::KnownOrigin::Store,
        }
    }

    fn run(ty: &str, name: &str) -> m::KnownEntity {
        m::KnownEntity {
            node_type: ty.into(),
            name: name.into(),
            aliases: Vec::new(),
            origin: m::KnownOrigin::Run,
        }
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// A store of three nodes: `acme/api` a `Repository` also known as `repo:acme/api`, `Acme` an
    /// `Organization`, and `web` a `Service`.
    fn graph() -> (Value, Value) {
        let ontology = json!({"node_types": [
            {"id": "t-repo", "name": "Repository"},
            {"id": "t-org", "name": "Organization"},
            {"id": "t-svc", "name": "Service"}]});
        let snapshot = json!({"graph": {"graph": {"nodes": {
            "n1": {"id": "n1", "type_id": "t-repo", "canonical_name": "acme/api",
                   "aliases": ["acme/api", "repo:acme/api"]},
            "n2": {"id": "n2", "type_id": "t-org", "canonical_name": "Acme", "aliases": ["Acme"]},
            "n3": {"id": "n3", "type_id": "t-svc", "canonical_name": "web", "aliases": ["web"]},
            "n4": {"id": "n4", "type_id": "t-gone", "canonical_name": "acme", "aliases": []}}}}});
        (snapshot, ontology)
    }

    #[test]
    fn a_batch_names_its_keys_and_each_records_id_folded() {
        let batch = [
            issued("https://Example.org/a", Origin::Url),
            issued("/data/repos.jsonl#Acme/API", Origin::FileRecord),
            issued("forge:projects.list:acme/web", Origin::Record),
        ];
        assert_eq!(
            terms(&batch),
            set(&[
                "https://example.org/a",
                "/data/repos.jsonl#acme/api",
                "acme/api",
                "forge:projects.list:acme/web",
                "acme/web",
            ])
        );
    }

    #[test]
    fn the_store_is_looked_up_across_types_by_name_or_alias_case_folded() {
        let (snapshot, ontology) = graph();
        assert_eq!(
            store_hits(&snapshot, &ontology, &set(&["acme/api", "acme"])),
            vec![
                store("Repository", "acme/api", &["repo:acme/api"]),
                store("Organization", "Acme", &[]),
            ]
        );
        assert_eq!(
            store_hits(&snapshot, &ontology, &set(&["repo:acme/api"])),
            vec![store("Repository", "acme/api", &["repo:acme/api"])]
        );
        assert_eq!(store_hits(&snapshot, &ontology, &set(&["api"])), vec![]);
        assert_eq!(store_hits(&json!({}), &ontology, &set(&["acme"])), vec![]);
    }

    #[test]
    fn a_template_alias_finds_the_store_node_of_that_alias() {
        let (snapshot, ontology) = graph();
        let template = crate::structured::render_template(
            "repo:{namespace}/{path}",
            &json!({"namespace": "acme", "path": "api"}),
        )
        .expect("rendered");
        assert_eq!(template, "repo:acme/api");
        assert_eq!(
            store_hits(&snapshot, &ontology, &set(&[&fold(&template)])),
            vec![store("Repository", "acme/api", &["repo:acme/api"])]
        );
    }

    #[test]
    fn store_hits_come_first_and_the_cap_holds_for_run_names_only() {
        let names: Vec<String> = (0..60).map(|n| format!("r{n}")).collect();
        let mut by_type = BTreeMap::from([("Repository".to_string(), names)]);
        by_type.insert("Person".into(), vec!["Ada".into(), "ACME/API".into()]);
        by_type
            .get_mut("Repository")
            .unwrap()
            .insert(0, "ACME/api".into());
        let merged = merge(
            vec![store("Repository", "acme/api", &["repo:acme/api"])],
            &by_type,
        );
        assert_eq!(
            merged[0],
            store("Repository", "acme/api", &["repo:acme/api"])
        );
        let repos: Vec<_> = merged
            .iter()
            .filter(|e| e.node_type == "Repository" && e.origin == m::KnownOrigin::Run)
            .collect();
        assert_eq!(repos.len(), RUN_PER_TYPE);
        assert_eq!(repos[0], &run("Repository", "r0"));
        assert!(
            merged.contains(&run("Person", "ACME/API")),
            "another type's name stays"
        );
        assert!(!merged.contains(&run("Repository", "ACME/api")));
    }

    #[test]
    fn a_store_hit_shows_its_origin_and_aliases_and_a_run_name_shows_as_before() {
        let shown = lines(&[
            store("Repository", "acme/api", &["repo:acme/api", "api"]),
            store("Organization", "Acme", &[]),
            run("Repository", "billing"),
            run("Person", "Ada"),
        ]);
        assert_eq!(
            shown,
            [
                "- Organization: Acme (in the store)",
                "- Person: Ada",
                "- Repository: acme/api (in the store; also repo:acme/api, api); billing",
            ]
        );
    }
}

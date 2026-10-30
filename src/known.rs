//! The known entities a run shows the model before a batch (`cortex.instance.KnownEntity`): the
//! store's nodes, of any type, that one of the batch's keys names by their canonical name or an
//! alias (origin `Store`), ahead of the names the run's own batches extracted (origin `Run`, at
//! most [`RUN_PER_TYPE`] per node type). The cap holds for run names only, so every store hit of
//! the batch is shown. No noun is persisted: a run reads the store once ([`Lookup`]: one `ekr
//! snapshot` at its first batch), and grows what it read by what each of its batches applied.

use std::collections::{BTreeMap, BTreeSet};

use cortex_model::instance as m;
use serde_json::Value;

use crate::evidence::Issued;
use crate::sources::Origin;

/// Known entity names per node type, of origin `Run`, given to the model.
pub const RUN_PER_TYPE: usize = 50;

/// How a name is compared: lowercased (`str::to_lowercase`, not full case folding: `ß` stays `ß`).
pub fn fold(text: &str) -> String {
    text.to_lowercase()
}

/// The terms of `batch` a store node may be named by, each [`fold`]ed: each document's key and, for
/// a record, its id: after the first `#` of a file record's key `<path>#<id>` (`src/sources.rs`
/// writes the path, then `#`, then the id, so an id holding `#` stays whole), after
/// `<adapter>:<operation>:` of a Connectors record's.
pub fn terms(batch: &[Issued]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for issued in batch {
        let key = &issued.doc.key;
        out.insert(fold(key));
        let id = match issued.doc.origin {
            Origin::FileRecord => key.split_once('#').map(|(_, id)| id),
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
    Index::from_snapshot(snapshot, ontology).hits(terms)
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

/// `hits` with every credential shape in their names and aliases masked (`crate::mask`), as a
/// document's text is on every run, with or without a redaction policy: a store node's name and
/// aliases are another writer's text. An alias that masks to the name or to an earlier alias is
/// kept once.
pub fn masked(hits: Vec<m::KnownEntity>) -> Vec<m::KnownEntity> {
    hits.into_iter()
        .map(|e| {
            let name = crate::mask::mask(&e.name).0;
            let mut aliases: Vec<String> = Vec::new();
            for alias in &e.aliases {
                let alias = crate::mask::mask(alias).0;
                if alias != name && !aliases.contains(&alias) {
                    aliases.push(alias);
                }
            }
            m::KnownEntity { name, aliases, ..e }
        })
        .collect()
}

/// The store's nodes as known entities of origin `Store`, each with its exact names, looked up by
/// their [`fold`]ed canonical name and aliases: what [`store_hits`] finds in one snapshot, kept for
/// a run and grown by what it applies ([`Index::applied`]).
#[derive(Debug, Default)]
pub struct Index {
    /// In the snapshot's node order, then in the order applied.
    nodes: Vec<m::KnownEntity>,
    /// Each folded canonical name and alias, and the positions in `nodes` it names.
    by_term: BTreeMap<String, Vec<usize>>,
}

impl Index {
    /// Every node of `snapshot` (`ekr snapshot`) of a type `ontology` (`ekr ontology`) names.
    pub fn from_snapshot(snapshot: &Value, ontology: &Value) -> Self {
        let types: BTreeMap<&str, &str> = ontology["node_types"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| Some((t["id"].as_str()?, t["name"].as_str()?)))
            .collect();
        let mut index = Self::default();
        for node in snapshot["graph"]["graph"]["nodes"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(_, n)| n)
        {
            let Some(name) = node["canonical_name"].as_str() else {
                continue;
            };
            let Some(node_type) = node["type_id"].as_str().and_then(|id| types.get(id)) else {
                continue;
            };
            let aliases = node["aliases"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str);
            index.add(node_type, name, aliases);
        }
        index
    }

    /// A node of type `node_type` named `name`, also known as `aliases` (`name` among them or not).
    fn add<'a>(&mut self, node_type: &str, name: &str, aliases: impl IntoIterator<Item = &'a str>) {
        let mut others: Vec<String> = Vec::new();
        for alias in aliases {
            if alias != name && !others.iter().any(|o| o == alias) {
                others.push(alias.to_string());
            }
        }
        let at = self.nodes.len();
        for term in std::iter::once(name).chain(others.iter().map(String::as_str)) {
            let positions = self.by_term.entry(fold(term)).or_default();
            if positions.last() != Some(&at) {
                positions.push(at);
            }
        }
        self.nodes.push(m::KnownEntity {
            node_type: node_type.to_string(),
            name: name.to_string(),
            aliases: others,
            origin: m::KnownOrigin::Store,
        });
    }

    /// Every node one of `terms` names, each once, in the index's order ([`store_hits`]).
    pub fn hits(&self, terms: &BTreeSet<String>) -> Vec<m::KnownEntity> {
        let at: BTreeSet<usize> = terms
            .iter()
            .filter_map(|t| self.by_term.get(t))
            .flatten()
            .copied()
            .collect();
        at.into_iter().map(|i| self.nodes[i].clone()).collect()
    }

    /// Adds the nodes `ekr apply-extraction` created applying `document` (the
    /// `ekr.extraction-document/1` the run wrote), as its `report` (`ekr.integrate.ExtractionReport`)
    /// tells. The report names no node, so the named things are read from the document as the verb
    /// resolves them: each entity, then each fact's subject and object, less those the report lists
    /// under `rejected` (`entities[i]`, `facts[i]`, `facts[i].subject`, `facts[i].object`); those of
    /// one node type that share an alias, directly or through another, are one thing; one with an
    /// alias equal to a name of an indexed node of its type is that node, and adds nothing; any
    /// other is a new node, named by the least, in byte order, of its members' first aliases, with
    /// every alias of them. A foreign writer's commit during the run is not seen.
    pub fn applied(&mut self, document: &serde_yaml_ng::Value, report: &Value) {
        use serde_yaml_ng::Value as Yaml;
        let rejected: BTreeSet<&str> = report["rejected"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r["item"].as_str())
            .collect();
        let named = |v: &Yaml| -> Option<(String, Vec<String>)> {
            let ty = v.get("node_type")?.as_str()?.to_string();
            let aliases: Vec<String> = v
                .get("aliases")?
                .as_sequence()?
                .iter()
                .filter_map(|a| a.as_str())
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect();
            (!aliases.is_empty()).then_some((ty, aliases))
        };
        let mut things: Vec<(String, Vec<String>)> = Vec::new();
        let entities = document.get("entities").and_then(Yaml::as_sequence);
        for (i, e) in entities.into_iter().flatten().enumerate() {
            if !rejected.contains(format!("entities[{i}]").as_str()) {
                things.extend(named(e));
            }
        }
        let facts = document.get("facts").and_then(Yaml::as_sequence);
        for (i, fact) in facts.into_iter().flatten().enumerate() {
            if rejected.contains(format!("facts[{i}]").as_str()) {
                continue;
            }
            let fact = match fact {
                Yaml::Tagged(t) => &t.value,
                other => other,
            };
            for end in ["subject", "object"] {
                if rejected.contains(format!("facts[{i}].{end}").as_str()) {
                    continue;
                }
                things.extend(fact.get(end).and_then(named));
            }
        }
        // One thing per node type and alias, transitively: each group's members by position.
        let mut groups: Vec<(String, Vec<String>, BTreeSet<String>)> = Vec::new();
        for (ty, aliases) in things {
            let first = aliases[0].clone();
            let mut all: BTreeSet<String> = aliases.into_iter().collect();
            let mut firsts = vec![first];
            let mut i = 0;
            while i < groups.len() {
                if groups[i].0 == ty && !groups[i].2.is_disjoint(&all) {
                    let (_, f, a) = groups.swap_remove(i);
                    firsts.extend(f);
                    all.extend(a);
                } else {
                    i += 1;
                }
            }
            groups.push((ty, firsts, all));
        }
        for (ty, firsts, all) in groups {
            let held = all.iter().any(|alias| {
                self.by_term.get(&fold(alias)).is_some_and(|at| {
                    at.iter().any(|&n| {
                        let node = &self.nodes[n];
                        node.node_type == ty
                            && (node.name == *alias || node.aliases.contains(alias))
                    })
                })
            });
            if held {
                continue;
            }
            let name = firsts.iter().min().expect("one member").clone();
            self.add(&ty, &name, all.iter().map(String::as_str));
        }
    }
}

/// The known entities a run shows the model, of one read of the store: the [`Index`] is built from
/// one `ekr snapshot` at the first batch that names anything, kept for the run and grown after
/// each apply ([`Lookup::applied`]).
#[derive(Debug, Default)]
pub struct Lookup {
    index: Option<Index>,
}

impl Lookup {
    /// The known entities of `batch`: the index's hits of its [`terms`], [`masked`], then
    /// [`merge`]d with the run's own names `run`. The first call with a term reads `store`.
    pub fn for_batch(
        &mut self,
        store: &crate::ekr::Store,
        ontology: &Value,
        batch: &[Issued],
        run: &BTreeMap<String, Vec<String>>,
    ) -> Result<Vec<m::KnownEntity>, String> {
        let terms = terms(batch);
        let hits = if terms.is_empty() {
            Vec::new()
        } else {
            if self.index.is_none() {
                self.index = Some(Index::from_snapshot(&store.snapshot()?, ontology));
            }
            self.index.as_ref().expect("built above").hits(&terms)
        };
        Ok(merge(masked(hits), run))
    }

    /// [`Index::applied`], once the index is built; before, the snapshot that builds it holds the
    /// document.
    pub fn applied(&mut self, document: &serde_yaml_ng::Value, report: &Value) {
        if let Some(index) = &mut self.index {
            index.applied(document, report);
        }
    }
}

/// `text` on one line: each line break (`\n`, `\r`, U+2028, U+2029) and every other control
/// character written as a space, so a name cannot add a line to the prompt.
pub fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// The known-entity lines of a prompt, one per node type in name order: `- <type>: ` and each
/// entity of it, store hits first, separated by `; `. A run name is written as it is; a store hit
/// as `<name> (in the store; also <alias>, <alias>)`. Every type, name and alias is written
/// [`one_line`], so each line is one type's.
pub fn lines(entities: &[m::KnownEntity]) -> Vec<String> {
    let mut by_type: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for e in entities {
        let (store, run) = by_type.entry(one_line(&e.node_type)).or_default();
        let name = one_line(&e.name);
        match e.origin {
            m::KnownOrigin::Store if e.aliases.is_empty() => {
                store.push(format!("{name} (in the store)"));
            }
            m::KnownOrigin::Store => {
                let aliases: Vec<String> = e.aliases.iter().map(|a| one_line(a)).collect();
                store.push(format!(
                    "{name} (in the store; also {})",
                    aliases.join(", ")
                ))
            }
            m::KnownOrigin::Run => run.push(name),
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
    fn a_batch_names_its_keys_and_each_records_id_lowercased() {
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
    fn the_store_is_looked_up_across_types_by_name_or_alias_lowercased() {
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

    /// `story:one-store-read-per-run`: what a batch applied is found by the next batch's terms as
    /// a fresh snapshot would find it, without reading the store again.
    #[test]
    fn an_applied_document_grows_the_index_as_ekr_resolves_its_named_things() {
        let (snapshot, ontology) = graph();
        let mut index = Index::from_snapshot(&snapshot, &ontology);
        let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            "entities:\n\
             - {node_type: Service, aliases: [zeta, billing]}\n\
             - {node_type: Service, aliases: [alpha-svc, billing]}\n\
             - {node_type: Repository, aliases: [repo:acme/api, other]}\n\
             - {node_type: Service, aliases: [refused]}\n\
             - {node_type: Organization, aliases: [web]}\n\
             facts:\n\
             - !Relation {subject: {node_type: Person, aliases: [Ada]}, relation: OWNS, \
               object: {node_type: Service, aliases: [gone]}, evidence: [e]}\n\
             - !Property {subject: {node_type: Person, aliases: [Bob]}, property: p, \
               value: {value_kind: String, value: v}, evidence: [e]}\n",
        )
        .unwrap();
        let report = json!({"rejected": [{"item": "entities[3]"}, {"item": "facts[0].object"},
                                {"item": "facts[1]"}]});
        index.applied(&document, &report);

        // Two entities sharing `billing` are one node, named by the least first alias.
        assert_eq!(
            index.hits(&set(&["billing"])),
            vec![store("Service", "alpha-svc", &["billing", "zeta"])]
        );
        // An alias of a node of its type held already resolves to that node: nothing is added.
        assert_eq!(
            index.hits(&set(&["other", "repo:acme/api"])),
            vec![store("Repository", "acme/api", &["repo:acme/api"])]
        );
        // The same name of another type is a node of its own.
        assert_eq!(
            index.hits(&set(&["web"])),
            vec![
                store("Service", "web", &[]),
                store("Organization", "web", &[])
            ]
        );
        // A fact's subject is a named thing; a rejected part names nothing.
        assert_eq!(
            index.hits(&set(&["ada"])),
            vec![store("Person", "Ada", &[])]
        );
        assert_eq!(index.hits(&set(&["refused", "gone", "bob"])), vec![]);
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

    #[test]
    fn a_store_hit_is_masked_for_credential_shapes_and_an_alias_masked_twice_is_kept_once() {
        // Assembled at runtime so no credential-shaped literal sits in the source.
        let one = format!("gh{}_{}", "p", "Ab1".repeat(13));
        let two = format!("gh{}_{}", "p", "Cd2".repeat(13));
        let hit = masked(vec![store("Repository", "acme/api", &[&one, &two, "api"])]);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].name, "acme/api");
        assert!(
            hit[0]
                .aliases
                .iter()
                .all(|a| !a.contains(&one) && !a.contains(&two)),
            "{hit:?}"
        );
        assert_eq!(hit[0].aliases.len(), 2, "{hit:?}");
        assert_eq!(hit[0].aliases[1], "api");
    }

    #[test]
    fn a_line_break_or_a_control_character_in_a_type_name_or_alias_is_written_as_a_space() {
        let shown = lines(&[
            store(
                "Repo\nsitory",
                "acme\r\napi",
                &["x\u{2028}y", "a\tb\u{85}c"],
            ),
            run("Repository", "bill\ning"),
        ]);
        assert_eq!(
            shown,
            [
                "- Repo sitory: acme  api (in the store; also x y, a b c)",
                "- Repository: bill ing",
            ]
        );
    }
}

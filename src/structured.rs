//! A `structured` source: each record becomes an entity, its properties and its relations by the
//! source's `mapping`, with no model call.
//!
//! A record is fetched as one document whose text is the record's JSON (`src/sources.rs`).
//! [`Source::prepare`] then gives it its identity, `<adapter>:<operation>:<id>`, as its key, and
//! masks credential shapes and applies the redaction policy's irreversible rules in every string of
//! it ([`clean`]); no model sees it, so nothing is pseudonymised. Each batch of records is one
//! `ekr.extraction-document/1` ([`Source::document`]) in the JSON form the model path answers in,
//! so `extract::merge` adds the evidence items cortex issued; every fact cites the evidence id of
//! the record it comes from.
//!
//! EKR resolves every alias of a named thing: things of one node type that share any alias are one
//! thing, named by the least first alias (`ekr` 0.0.30 `docs/cli.md`, `apply-extraction`). So a
//! record's aliases are `<name> (<identity>)`, which names the node, the identity, and the mapped
//! aliases, which resolve across records on purpose. The bare name and the bare id are not
//! aliases: two people of one name, or one record's id equal to another's handle, stay apart.

use std::collections::{BTreeMap, BTreeSet};

use cortex_model::instance as m;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::evidence::Issued;
use crate::mask::mask;
use crate::redact::Redactor;
use crate::sources::{at, Document};

/// A scalar as text: a string as it is, a number or a boolean as JSON writes it; nothing for a
/// null, an empty string, an array or an object.
fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(t) if !t.trim().is_empty() => Some(t.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The text at `path` of `record`, when it is a scalar.
pub fn text_at(record: &Value, path: &str) -> Option<String> {
    at(record, path).and_then(scalar)
}

/// The texts at `path` of `record`: the scalar there, or each scalar of the array there.
fn texts_at(record: &Value, path: &str) -> Vec<String> {
    match at(record, path) {
        Some(Value::Array(items)) => items.iter().filter_map(scalar).collect(),
        Some(v) => scalar(v).into_iter().collect(),
        None => Vec::new(),
    }
}

/// `<adapter>:<operation>` of a structured source's input; a `files` input, which does not run
/// yet, is `files:<glob>`.
pub fn prefix(st: &m::StructuredSource) -> String {
    match &st.input {
        m::StructuredInput::Connectors(c) => format!("{}:{}", c.adapter, c.operation),
        m::StructuredInput::Files(f) => format!("files:{}", f.glob),
    }
}

/// `text`, a record's JSON, with credential shapes masked and the irreversible rules of
/// `redactor` applied in every string value; how many credentials were masked; and how many
/// matches each irreversible rule replaced. A text that is not JSON is masked as a whole.
pub fn clean(text: &str, redactor: Option<&Redactor>) -> (String, usize, BTreeMap<String, usize>) {
    let mut masked = 0;
    let mut scrubbed = BTreeMap::new();
    let Ok(mut record) = serde_json::from_str::<Value>(text) else {
        let (text, n) = mask(text);
        return (text, n, scrubbed);
    };
    clean_value(&mut record, redactor, &mut masked, &mut scrubbed);
    (record.to_string(), masked, scrubbed)
}

fn clean_value(
    v: &mut Value,
    redactor: Option<&Redactor>,
    masked: &mut usize,
    scrubbed: &mut BTreeMap<String, usize>,
) {
    match v {
        Value::String(t) => {
            let (text, n) = mask(t);
            *masked += n;
            *t = text;
            if let Some(r) = redactor {
                let (text, hits) = r.scrub(t);
                *t = text;
                for (name, _) in hits {
                    *scrubbed.entry(name).or_default() += 1;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                clean_value(item, redactor, masked, scrubbed);
            }
        }
        Value::Object(fields) => {
            for item in fields.values_mut() {
                clean_value(item, redactor, masked, scrubbed);
            }
        }
        _ => {}
    }
}

fn string_property(name: &str) -> Value {
    json!({"name": name, "value": {"value_kind": "String"}, "cardinality": "One", "required": false})
}

/// The ontology a mapping writes: its node type with every mapped property as a `String`, each
/// relation's target type, and each relation as an edge type from the node type to its target.
/// `apply-extraction` adds only what the store does not hold yet.
fn ontology(mapping: &m::RecordMapping) -> Value {
    let mut properties: Vec<&str> = Vec::new();
    for p in &mapping.properties {
        if !properties.contains(&p.property.as_str()) {
            properties.push(&p.property);
        }
    }
    let mut node_types = vec![json!({
        "name": mapping.node_type,
        "parents": [],
        "abstract_type": false,
        "properties": properties.iter().map(|p| string_property(p)).collect::<Vec<_>>(),
    })];
    let mut declared = vec![mapping.node_type.as_str()];
    let mut edge_types = Vec::new();
    for r in &mapping.relations {
        if !declared.contains(&r.target_type.as_str()) {
            declared.push(&r.target_type);
            node_types.push(json!({
                "name": r.target_type, "parents": [], "abstract_type": false, "properties": [],
            }));
        }
        edge_types.push(json!({
            "name": r.relation,
            "source_types": [mapping.node_type],
            "target_types": [r.target_type],
            "cardinality": "Many",
            "properties": [],
        }));
    }
    json!({"node_types": node_types, "edge_types": edge_types})
}

/// One structured source as a run maps it: its identity prefix, its mapping and the instance's
/// redaction policy.
pub struct Source<'a> {
    /// `<adapter>:<operation>` ([`prefix`]).
    pub prefix: String,
    pub mapping: &'a m::RecordMapping,
    pub redactor: Option<&'a Redactor>,
}

/// One record of a run, as relation targets find it.
struct Indexed {
    identity: String,
    /// The record's name and mapped aliases, as a relation target may name it.
    names: Vec<String>,
    /// Every alias of the record's entity.
    aliases: Vec<String>,
}

/// The records of one run, so a relation names a target of the run by all its aliases, whichever
/// batch the target is in.
pub struct Index(Vec<Indexed>);

/// One batch's extraction document, and the evidence ids of the records each of its entities
/// stands for or was named by, in the document's order.
pub struct Mapped {
    pub document: Value,
    pub entity_owners: Vec<Vec<String>>,
}

impl<'a> Source<'a> {
    pub fn new(st: &'a m::StructuredSource, redactor: Option<&'a Redactor>) -> Self {
        Self {
            prefix: prefix(st),
            mapping: &st.mapping,
            redactor,
        }
    }

    /// Whether masking or an irreversible rule would change `text`.
    fn cleaning_changes(&self, text: &str) -> bool {
        mask(text).0 != text || self.redactor.is_some_and(|r| r.scrub(text).0 != text)
    }

    /// The identity of the record whose raw id is `raw_id`: `<adapter>:<operation>:<id>`, or,
    /// when masking or an irreversible rule would change the id or that identity,
    /// `<adapter>:<operation>:` and the first 16 hex digits of the id's SHA-256, so the id itself
    /// is never stored and two ids cleaned to one text stay two identities.
    pub fn identity(&self, raw_id: &str) -> String {
        let plain = format!("{}:{raw_id}", self.prefix);
        if !self.cleaning_changes(raw_id) && !self.cleaning_changes(&plain) {
            return plain;
        }
        let digest = crate::state::hex(&Sha256::digest(raw_id.as_bytes()));
        format!("{}:{}", self.prefix, &digest[..16])
    }

    /// Readies a fetched record: its key becomes the identity of its raw id, and its text is
    /// [`clean`]ed. Answers how many credentials were masked and what each irreversible rule
    /// replaced.
    pub fn prepare(&self, doc: &mut Document) -> (usize, BTreeMap<String, usize>) {
        let raw_id = serde_json::from_str::<Value>(&doc.text)
            .ok()
            .and_then(|r| text_at(&r, &self.mapping.id))
            .unwrap_or_default();
        doc.key = self.identity(&raw_id);
        let (text, masked, scrubbed) = clean(&doc.text, self.redactor);
        doc.text = text;
        (masked, scrubbed)
    }

    /// The record's entity aliases, given its identity: `<name> (<identity>)` first, so it names
    /// the node, then the identity, then the mapped aliases, each once. `None` without a name.
    fn aliases(&self, record: &Value, identity: &str) -> Option<Vec<String>> {
        let name = text_at(record, &self.mapping.name)?;
        let mut out = vec![format!("{name} ({identity})"), identity.to_string()];
        for alias in self
            .mapping
            .aliases
            .iter()
            .flat_map(|p| texts_at(record, p))
        {
            if !out.contains(&alias) {
                out.push(alias);
            }
        }
        Some(out)
    }

    /// The run's records, whose keys are their identities ([`Source::prepare`]).
    pub fn index(&self, records: &[Issued]) -> Index {
        Index(
            records
                .iter()
                .filter_map(|issued| {
                    let record: Value = serde_json::from_str(&issued.doc.text).ok()?;
                    let aliases = self.aliases(&record, &issued.doc.key)?;
                    let names = text_at(&record, &self.mapping.name)
                        .into_iter()
                        .chain(
                            self.mapping
                                .aliases
                                .iter()
                                .flat_map(|p| texts_at(&record, p)),
                        )
                        .collect();
                    Some(Indexed {
                        identity: issued.doc.key.clone(),
                        names,
                        aliases,
                    })
                })
                .collect(),
        )
    }

    /// The named thing a relation's `target` text names, of `node_type`. Of the mapping's own node
    /// type, it is the run's record whose identity is the target's as an id, else the one record of
    /// the run with it as its name or a mapped alias, named by all that record's aliases; else
    /// the target and its identity as an id, so it reaches a record of an earlier run by its id or
    /// by a mapped alias. Of another type, the target alone.
    fn target(&self, index: &Index, node_type: &str, target: &str) -> Value {
        if node_type != self.mapping.node_type {
            return json!({"node_type": node_type, "aliases": [target]});
        }
        let identity = self.identity(target);
        let by_id = index.0.iter().find(|r| r.identity == identity);
        let by_name = || {
            let mut named = index
                .0
                .iter()
                .filter(|r| r.names.iter().any(|n| n == target));
            match (named.next(), named.next()) {
                (Some(one), None) => Some(one),
                _ => None,
            }
        };
        match by_id.or_else(by_name) {
            Some(r) => json!({"node_type": node_type, "aliases": r.aliases}),
            None => json!({"node_type": node_type, "aliases": [target, identity]}),
        }
    }

    /// The `ekr.extraction-document/1` of one batch of records, in the JSON form the model path
    /// answers in (`"!Property"` keys), without its evidence: `extract::merge` adds the items `batch`
    /// was issued. Each record whose text is JSON and has a name at the mapping's `name` is one
    /// entity of the mapping's node type, named by its aliases; each mapped property with a scalar
    /// value is a `String` property; each relation target named at `target_name` (a scalar, or
    /// each scalar of an array) is a relation to the named thing [`Source::target`] answers. Every
    /// fact cites the evidence id of its record.
    pub fn document(&self, batch: &[Issued], index: &Index) -> Mapped {
        let mapping = self.mapping;
        let mut entities: Vec<Value> = Vec::new();
        let mut entity_owners: Vec<Vec<String>> = Vec::new();
        let mut entity = |e: Value, owner: &str| {
            let at = match entities.iter().position(|x| *x == e) {
                Some(at) => at,
                None => {
                    entities.push(e);
                    entity_owners.push(Vec::new());
                    entities.len() - 1
                }
            };
            if !entity_owners[at].iter().any(|o| o == owner) {
                entity_owners[at].push(owner.to_string());
            }
        };
        let mut facts = Vec::new();
        for issued in batch {
            let Ok(record) = serde_json::from_str::<Value>(&issued.doc.text) else {
                continue;
            };
            let Some(aliases) = self.aliases(&record, &issued.doc.key) else {
                continue;
            };
            let subject = json!({"node_type": mapping.node_type, "aliases": aliases});
            entity(subject.clone(), &issued.id);
            for p in &mapping.properties {
                if let Some(value) = text_at(&record, &p.path) {
                    facts.push(json!({"!Property": {
                        "subject": subject,
                        "property": p.property,
                        "value": {"value_kind": "String", "value": value},
                        "evidence": [issued.id],
                    }}));
                }
            }
            for r in &mapping.relations {
                for target in texts_at(&record, &r.target_name) {
                    let object = self.target(index, &r.target_type, &target);
                    entity(object.clone(), &issued.id);
                    facts.push(json!({"!Relation": {
                        "subject": subject,
                        "relation": r.relation,
                        "object": object,
                        "evidence": [issued.id],
                    }}));
                }
            }
        }
        Mapped {
            document: json!({
                "format": "ekr.extraction-document/1",
                "ontology": ontology(mapping),
                "entities": entities,
                "facts": facts,
            }),
            entity_owners,
        }
    }
}

impl Mapped {
    /// The evidence ids of the records with a part `report` (an `ekr.integrate.ExtractionReport`)
    /// lists as `rejected`: a fact's record, every record an entity stands for or was named by,
    /// and, for the `ontology` or an item it does not know, every record of the batch.
    pub fn rejected(&self, report: &Value) -> BTreeSet<String> {
        let cited = |fact: &Value| -> Vec<String> {
            fact.as_object()
                .and_then(|o| o.values().next())
                .map(|body| {
                    body["evidence"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        let facts = self.document["facts"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let every = || -> BTreeSet<String> {
            facts
                .iter()
                .flat_map(cited)
                .chain(self.entity_owners.iter().flatten().cloned())
                .collect()
        };
        let index = |item: &str, list: &str| -> Option<usize> {
            item.strip_prefix(list)?
                .strip_prefix('[')?
                .split(']')
                .next()?
                .parse()
                .ok()
        };
        let mut out = BTreeSet::new();
        for part in report["rejected"].as_array().into_iter().flatten() {
            let item = part["item"].as_str().unwrap_or_default();
            if let Some(i) = index(item, "facts") {
                if let Some(fact) = facts.get(i) {
                    out.extend(cited(fact));
                    continue;
                }
            } else if let Some(i) = index(item, "entities") {
                if let Some(owners) = self.entity_owners.get(i) {
                    out.extend(owners.iter().cloned());
                    continue;
                }
            }
            out.extend(every());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Origin;

    fn mapping() -> m::RecordMapping {
        m::RecordMapping {
            node_type: "Person".into(),
            id: "$.id".into(),
            name: "$.name".into(),
            aliases: vec!["$.handles".into()],
            properties: vec![m::PropertyMapping {
                property: "age".into(),
                path: "$.age".into(),
            }],
            relations: vec![m::RelationMapping {
                relation: "MEMBER_OF".into(),
                target_type: "Team".into(),
                target_name: "$.teams".into(),
            }],
        }
    }

    fn source(mapping: &m::RecordMapping) -> Source<'_> {
        Source {
            prefix: "dir:people.list".into(),
            mapping,
            redactor: None,
        }
    }

    fn issued(s: &Source<'_>, id: &str, record: Value) -> Issued {
        let mut doc = Document {
            key: String::new(),
            origin: Origin::Record,
            title: None,
            description: None,
            published: None,
            text: record.to_string(),
            hash: None,
        };
        s.prepare(&mut doc);
        Issued {
            id: id.into(),
            item: serde_yaml_ng::Value::Null,
            doc,
        }
    }

    #[test]
    fn a_record_maps_to_one_entity_its_properties_and_a_relation_per_target_each_citing_its_evidence(
    ) {
        let m = mapping();
        let s = source(&m);
        let batch = [
            issued(
                &s,
                "e1",
                json!({"id": 7, "name": "Ada", "handles": ["ada", "Ada"], "age": 36, "teams": ["Core", "Ops"]}),
            ),
            issued(&s, "e2", json!({"id": "8", "handles": "nameless"})),
        ];
        let mapped = s.document(&batch, &s.index(&batch));
        let doc = &mapped.document;
        let ada = json!({"node_type": "Person",
            "aliases": ["Ada (dir:people.list:7)", "dir:people.list:7", "ada", "Ada"]});
        assert_eq!(
            doc["entities"],
            json!([ada, {"node_type": "Team", "aliases": ["Core"]}, {"node_type": "Team", "aliases": ["Ops"]}])
        );
        assert_eq!(mapped.entity_owners, [["e1"], ["e1"], ["e1"]]);
        let facts = doc["facts"].as_array().unwrap();
        assert_eq!(facts.len(), 3, "{doc}");
        assert_eq!(
            facts[0],
            json!({"!Property": {"subject": ada, "property": "age",
                "value": {"value_kind": "String", "value": "36"}, "evidence": ["e1"]}})
        );
        assert_eq!(facts[2]["!Relation"]["object"]["aliases"], json!(["Ops"]));
        assert_eq!(
            doc["ontology"]["edge_types"][0],
            json!({"name": "MEMBER_OF", "source_types": ["Person"], "target_types": ["Team"],
                "cardinality": "Many", "properties": []})
        );
    }

    #[test]
    fn a_relation_names_a_record_of_the_run_by_all_its_aliases_and_any_other_by_text_and_identity()
    {
        let mut m = mapping();
        m.relations[0] = m::RelationMapping {
            relation: "REPORTS_TO".into(),
            target_type: "Person".into(),
            target_name: "$.manager".into(),
        };
        let s = source(&m);
        let ada = issued(&s, "e1", json!({"id": "P-1", "name": "Ada"}));
        let grace = issued(
            &s,
            "e2",
            json!({"id": "P-2", "name": "Grace", "manager": ["P-1", "Ada", "P-9"]}),
        );
        let index = s.index(&[ada, grace]);
        let batch = [issued(
            &s,
            "e2",
            json!({"id": "P-2", "name": "Grace", "manager": ["P-1", "Ada", "P-9"]}),
        )];
        let doc = s.document(&batch, &index).document;
        let full = json!({"node_type": "Person", "aliases": ["Ada (dir:people.list:P-1)", "dir:people.list:P-1"]});
        assert_eq!(doc["facts"][0]["!Relation"]["object"], full, "by id");
        assert_eq!(doc["facts"][1]["!Relation"]["object"], full, "by name");
        assert_eq!(
            doc["facts"][2]["!Relation"]["object"],
            json!({"node_type": "Person", "aliases": ["P-9", "dir:people.list:P-9"]})
        );
        assert_eq!(doc["ontology"]["node_types"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn an_id_cleaning_would_change_is_stored_as_a_digest() {
        let m = mapping();
        let s = source(&m);
        let token = &format!("glpat-{}", "abcdefghijklmnopqrstuvwx");
        let one = s.identity(token);
        assert!(one.starts_with("dir:people.list:"), "{one}");
        assert!(!one.contains("glpat"), "{one}");
        assert_eq!(one.len(), "dir:people.list:".len() + 16);
        assert_ne!(one, s.identity(&format!("glpat-{}", "abcdefghijklmnopqrstuvwy")));
        assert_eq!(s.identity("P-1"), "dir:people.list:P-1");
    }

    #[test]
    fn a_rejected_part_names_the_records_it_belongs_to() {
        let m = mapping();
        let s = source(&m);
        let batch = [
            issued(
                &s,
                "e1",
                json!({"id": 1, "name": "Ada", "age": 1, "teams": "Core"}),
            ),
            issued(
                &s,
                "e2",
                json!({"id": 2, "name": "Bob", "age": 2, "teams": "Core"}),
            ),
            issued(&s, "e3", json!({"id": 3, "name": "Cy", "age": 3})),
        ];
        let mapped = s.document(&batch, &s.index(&batch));
        let rejected = |items: &[&str]| -> Vec<String> {
            let parts: Vec<_> = items.iter().map(|i| json!({"item": i})).collect();
            mapped
                .rejected(&json!({"rejected": parts}))
                .into_iter()
                .collect()
        };
        assert!(rejected(&[]).is_empty());
        assert_eq!(rejected(&["facts[0]"]), ["e1"]);
        assert_eq!(rejected(&["facts[2].object"]), ["e2"]);
        // The shared `Core` team entity was named by Ada and Bob.
        assert_eq!(rejected(&["entities[1]"]), ["e1", "e2"]);
        assert_eq!(rejected(&["ontology"]), ["e1", "e2", "e3"]);
        assert_eq!(rejected(&["something[0]"]), ["e1", "e2", "e3"]);
    }

    #[test]
    fn cleaning_masks_every_string_of_a_record_and_keeps_it_json() {
        let text =
            json!({"a": {"b": [format!("token glpat-{}", "abcdefghijklmnopqrstuvwx")]}, "n": 1}).to_string();
        let (cleaned, masked, scrubbed) = clean(&text, None);
        assert_eq!(masked, 1);
        assert!(scrubbed.is_empty());
        assert!(
            !cleaned.contains(&format!("glpat-{}", "abcdefghijklmnopqrstuvwx")),
            "{cleaned}"
        );
        let v: Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(v["n"], 1);
    }
}

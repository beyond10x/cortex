//! `story:figure-svg-renderer`: cortex draws a figure (`cortex.ask.FigureSpec`: a bar chart, a
//! timeline or a relationship graph) as SVG with its own code, from typed values only.
//!
//! Every output is read back with an XML parser that is not cortex's own (`roxmltree`) and held to
//! an allow-list written here, independently of the renderer: the elements `svg`, `g`, `rect`,
//! `line`, `path`, `circle`, `text` and `title`, geometry, fill, stroke and text attributes, no
//! label text in any attribute, and no processing instruction, comment or document type. The same
//! figure gives the same bytes, equal to a committed fixture, and a figure over a cap is refused
//! whole with a typed error.
//!
//! The fixtures under `tests/fixtures/figure/` are rewritten from the renderer when
//! `CORTEX_FIGURE_BLESS=1` is set; read the diff before committing one.

use std::path::PathBuf;

use cortex_cli::ask::figure::{
    render, FigureError, MAX_BARS, MAX_BYTES, MAX_EDGES, MAX_ENTRIES, MAX_NODES,
};
use cortex_model::ask as ma;
use cortex_model::primitives::Timestamp;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// The elements a figure may hold.
const ELEMENTS: &[&str] = &[
    "svg", "g", "rect", "line", "path", "circle", "text", "title",
];

/// The attributes a figure may hold: geometry, fill, stroke and text.
const ATTRIBUTES: &[&str] = &[
    "width",
    "height",
    "viewBox",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "d",
    "fill",
    "stroke",
    "stroke-width",
    "font-family",
    "font-size",
    "font-weight",
    "text-anchor",
];

fn bar(label: &str, count: i64) -> ma::Bar {
    ma::Bar {
        label: label.into(),
        count,
    }
}

fn entry(at: &str, label: &str) -> ma::TimelineEntry {
    ma::TimelineEntry {
        at: Timestamp(at.into()),
        label: label.into(),
    }
}

fn node(id: &str, label: &str) -> ma::GraphNode {
    ma::GraphNode {
        id: id.into(),
        label: label.into(),
    }
}

fn edge(source: &str, target: &str, label: &str) -> ma::GraphEdge {
    ma::GraphEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

fn bar_chart(title: &str, bars: Vec<ma::Bar>) -> ma::FigureSpec {
    ma::FigureSpec::BarChart(ma::BarChart {
        title: title.into(),
        bars,
    })
}

fn timeline(title: &str, entries: Vec<ma::TimelineEntry>) -> ma::FigureSpec {
    ma::FigureSpec::Timeline(ma::Timeline {
        title: title.into(),
        entries,
    })
}

fn graph(title: &str, nodes: Vec<ma::GraphNode>, edges: Vec<ma::GraphEdge>) -> ma::FigureSpec {
    ma::FigureSpec::RelationshipGraph(ma::RelationshipGraph {
        title: title.into(),
        nodes,
        edges,
    })
}

/// The text a label is drawn as: each control character, and each of U+FFFE and U+FFFF (no
/// XML character), as U+FFFD.
fn drawn(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_control() || c == '\u{FFFE}' || c == '\u{FFFF}' {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// Holds `svg` to the allow-list and returns the text of every `text` and `title` element, in
/// document order.
fn allowed(svg: &str) -> Vec<String> {
    assert!(
        svg.starts_with("<svg "),
        "a figure starts with its svg element, no prolog: {svg:.80}"
    );
    let doc = roxmltree::Document::parse(svg)
        .unwrap_or_else(|e| panic!("a figure is well-formed XML ({e}): {svg:.400}"));
    let mut texts = Vec::new();
    for n in doc.root().descendants() {
        match n.node_type() {
            roxmltree::NodeType::Root => {}
            roxmltree::NodeType::Element => {
                let name = n.tag_name();
                assert_eq!(
                    name.namespace(),
                    Some(SVG_NS),
                    "{:?} is in the SVG namespace",
                    name
                );
                assert!(
                    ELEMENTS.contains(&name.name()),
                    "element {:?} is outside the allow-list",
                    name.name()
                );
                for ns in n.namespaces() {
                    assert!(
                        ns.name().is_none() && ns.uri() == SVG_NS,
                        "the only namespace is SVG's, undeclared prefix: {ns:?}"
                    );
                }
                for a in n.attributes() {
                    assert!(
                        a.namespace().is_none(),
                        "attribute {:?} has a namespace",
                        a.name()
                    );
                    assert!(
                        ATTRIBUTES.contains(&a.name()),
                        "attribute {:?} on {:?} is outside the allow-list",
                        a.name(),
                        name.name()
                    );
                    assert!(
                        a.value().chars().all(|c| c.is_ascii_alphanumeric()
                            || matches!(c, ' ' | '.' | ',' | '-' | '#')),
                        "attribute {}={:?} holds more than numbers, colours and keywords",
                        a.name(),
                        a.value()
                    );
                }
                if matches!(name.name(), "text" | "title") {
                    assert!(
                        n.children().all(|c| c.is_text()),
                        "{:?} holds text only",
                        name.name()
                    );
                    texts.push(n.text().unwrap_or("").to_string());
                }
            }
            roxmltree::NodeType::Text => {
                let parent = n.parent_element().map(|p| p.tag_name().name());
                assert!(
                    matches!(parent, Some("text" | "title")),
                    "text outside a text or title element: {:?}",
                    n.text()
                );
            }
            other => panic!("a figure holds no {other:?} node"),
        }
    }
    texts
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/figure")
        .join(name)
}

/// `spec` renders, twice, to the bytes of the committed fixture `name`.
fn matches_fixture(spec: &ma::FigureSpec, name: &str) {
    let first = render(spec).expect("the fixed figure renders");
    let second = render(spec).expect("the fixed figure renders again");
    assert_eq!(first, second, "the same figure gives the same bytes");
    let path = fixture(name);
    if std::env::var("CORTEX_FIGURE_BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &first).unwrap();
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("the fixture {} is committed ({e})", path.display()));
    assert_eq!(
        first, committed,
        "{name}: the rendering equals the committed fixture"
    );
    allowed(&first);
}

fn fixed_bar_chart() -> ma::FigureSpec {
    bar_chart(
        "Open incidents by team",
        vec![
            bar("Payments", 12),
            bar("Search & <Discovery>", 7),
            bar("Identity", 0),
            bar("\"Platform\"", 3),
        ],
    )
}

fn fixed_timeline() -> ma::FigureSpec {
    timeline(
        "Release of 0.6.0",
        vec![
            entry("2026-10-17T09:30:00Z", "gate green"),
            entry("2026-10-16T22:00:00+02:00", "branch cut"),
            entry("2026-10-17T12:00:00.250Z", "tag & publish"),
        ],
    )
}

fn fixed_graph() -> ma::FigureSpec {
    graph(
        "Who decided what",
        vec![
            node("node:a1", "Ada"),
            node("node:b2", "Billing <service>"),
            node("evidence:c3", "Minutes 2026-10-01"),
            node("node:d4", "O'Neil"),
        ],
        vec![
            edge("node:a1", "node:b2", "owns"),
            edge("evidence:c3", "node:a1", "cites"),
            edge("node:d4", "node:b2", "reviews & signs"),
        ],
    )
}

#[test]
fn a_fixed_bar_chart_renders_to_its_fixture_twice() {
    matches_fixture(&fixed_bar_chart(), "bar-chart.svg");
}

#[test]
fn a_fixed_timeline_renders_to_its_fixture_twice() {
    matches_fixture(&fixed_timeline(), "timeline.svg");
}

#[test]
fn a_fixed_relationship_graph_renders_to_its_fixture_twice() {
    matches_fixture(&fixed_graph(), "relationship-graph.svg");
}

#[test]
fn every_kind_draws_its_title_and_labels_as_text() {
    let texts = allowed(&render(&fixed_bar_chart()).unwrap());
    for want in [
        "Open incidents by team",
        "Search & <Discovery>",
        "\"Platform\"",
        "12",
        "0",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want:?} in {texts:?}");
    }
    let texts = allowed(&render(&fixed_timeline()).unwrap());
    for want in ["Release of 0.6.0", "tag & publish", "2026-10-16T20:00:00Z"] {
        assert!(texts.iter().any(|t| t == want), "{want:?} in {texts:?}");
    }
    let texts = allowed(&render(&fixed_graph()).unwrap());
    for want in [
        "Who decided what",
        "Billing <service>",
        "O'Neil",
        "reviews & signs",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want:?} in {texts:?}");
    }
}

#[test]
fn labels_are_escaped_not_written_as_markup() {
    let svg = render(&bar_chart(
        "<script>alert(1)</script>",
        vec![bar(
            "a<b & \"c\" 'd' </text><script>x</script> onload=\"e()\"",
            1,
        )],
    ))
    .unwrap();
    assert!(!svg.contains("<script"), "{svg}");
    // The label's own `</text>` and handler, raw, would close its element and open markup.
    assert!(!svg.contains("'d' </text>"), "{svg}");
    assert!(!svg.contains("onload=\""), "{svg}");
    assert!(
        svg.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{svg}"
    );
    assert!(
        svg.contains("a&lt;b &amp; &quot;c&quot; &apos;d&apos; &lt;/text&gt;&lt;script&gt;"),
        "{svg}"
    );
    let texts = allowed(&svg);
    assert!(texts
        .iter()
        .any(|t| t == "a<b & \"c\" 'd' </text><script>x</script> onload=\"e()\""));
}

#[test]
fn control_characters_are_drawn_as_the_replacement_character() {
    let label = "nul\u{0}bell\u{7}tab\tnl\ndel\u{7f}c1\u{85}nc\u{FFFE}";
    let svg = render(&graph("t\u{1b}[31m", vec![node("n\u{0}1", label)], vec![])).unwrap();
    let texts = allowed(&svg);
    assert!(texts.contains(&drawn(label)), "{texts:?}");
    assert!(texts.contains(&drawn("t\u{1b}[31m")), "{texts:?}");
    assert!(!svg.chars().any(|c| c.is_control() && c != '\n'), "{svg:?}");
}

#[test]
fn a_timeline_is_in_order_of_its_instants_in_utc() {
    let texts = allowed(&render(&fixed_timeline()).unwrap());
    let at: Vec<&String> = texts.iter().filter(|t| t.starts_with("2026-")).collect();
    assert_eq!(
        at,
        [
            "2026-10-16T20:00:00Z",
            "2026-10-17T09:30:00Z",
            "2026-10-17T12:00:00Z"
        ]
    );
    let labels: Vec<&String> = texts
        .iter()
        .filter(|t| ["gate green", "branch cut", "tag & publish"].contains(&t.as_str()))
        .collect();
    assert_eq!(labels, ["branch cut", "gate green", "tag & publish"]);
}

fn too_large(spec: &ma::FigureSpec) -> ma::FigureTooLarge {
    match render(spec) {
        Err(FigureError::TooLarge(e)) => e,
        other => panic!("refused as too large, got {other:?}"),
    }
}

fn invalid(spec: &ma::FigureSpec) -> ma::FigureInvalid {
    match render(spec) {
        Err(FigureError::Invalid(e)) => e,
        other => panic!("refused as invalid, got {other:?}"),
    }
}

#[test]
fn the_caps_are_those_the_spec_states() {
    assert_eq!(
        (MAX_BARS, MAX_ENTRIES, MAX_NODES, MAX_EDGES, MAX_BYTES),
        (50, 200, 100, 300, 262_144)
    );
}

#[test]
fn a_figure_at_each_count_cap_renders() {
    let bars = (0..MAX_BARS)
        .map(|i| bar(&format!("b{i}"), i as i64))
        .collect();
    allowed(&render(&bar_chart("t", bars)).unwrap());
    let entries = (0..MAX_ENTRIES)
        .map(|i| entry(&format!("2026-01-01T00:00:{:02}Z", i % 60), "e"))
        .collect();
    allowed(&render(&timeline("t", entries)).unwrap());
    let nodes: Vec<_> = (0..MAX_NODES)
        .map(|i| node(&format!("n{i}"), "n"))
        .collect();
    let edges = (0..MAX_EDGES)
        .map(|i| {
            edge(
                &format!("n{}", i % MAX_NODES),
                &format!("n{}", (i * 7 + 1) % MAX_NODES),
                "e",
            )
        })
        .collect();
    allowed(&render(&graph("t", nodes, edges)).unwrap());
}

#[test]
fn a_figure_over_a_count_cap_is_refused_whole() {
    let bars = (0..=MAX_BARS).map(|i| bar("b", i as i64)).collect();
    let e = too_large(&bar_chart("t", bars));
    assert_eq!((e.limit.as_str(), e.max, e.actual), ("bars", 50, 51));

    let entries = (0..=MAX_ENTRIES)
        .map(|_| entry("2026-01-01T00:00:00Z", "e"))
        .collect();
    let e = too_large(&timeline("t", entries));
    assert_eq!((e.limit.as_str(), e.max, e.actual), ("entries", 200, 201));

    let nodes: Vec<_> = (0..=MAX_NODES)
        .map(|i| node(&format!("n{i}"), "n"))
        .collect();
    let e = too_large(&graph("t", nodes, vec![]));
    assert_eq!((e.limit.as_str(), e.max, e.actual), ("nodes", 100, 101));

    let nodes = vec![node("a", "a"), node("b", "b")];
    let edges = (0..=MAX_EDGES).map(|_| edge("a", "b", "e")).collect();
    let e = too_large(&graph("t", nodes, edges));
    assert_eq!((e.limit.as_str(), e.max, e.actual), ("edges", 300, 301));
}

#[test]
fn a_figure_over_the_byte_cap_is_refused_not_cut() {
    let long = "x".repeat(MAX_BYTES / 4);
    let bars = (0..5).map(|i| bar(&long, i)).collect();
    let e = too_large(&bar_chart("t", bars));
    assert_eq!((e.limit.as_str(), e.max), ("bytes", 262_144));
    assert!(e.actual > 262_144, "{}", e.actual);

    // Escaping counts: a label within the cap as text can be over it as markup.
    let amp = "&".repeat(MAX_BYTES / 4);
    let e = too_large(&bar_chart("t", vec![bar(&amp, 1)]));
    assert_eq!(e.limit, "bytes");
    assert!(e.actual > 262_144, "{}", e.actual);
}

#[test]
fn a_figure_with_a_value_it_cannot_be_drawn_from_is_refused() {
    let e = invalid(&bar_chart("t", vec![bar("a", 1), bar("b", -1)]));
    assert_eq!(e.field, "bars[1].count");

    let e = invalid(&timeline(
        "t",
        vec![entry("2026-10-17T00:00:00Z", "a"), entry("yesterday", "b")],
    ));
    assert_eq!(e.field, "entries[1].at");

    let e = invalid(&graph("t", vec![node("a", "a"), node("a", "b")], vec![]));
    assert_eq!(e.field, "nodes[1].id");

    let e = invalid(&graph(
        "t",
        vec![node("a", "a"), node("b", "b")],
        vec![edge("a", "b", "ok"), edge("a", "c", "dangling")],
    ));
    assert_eq!(e.field, "edges[1].target");

    let e = invalid(&graph("t", vec![node("a", "a")], vec![edge("z", "a", "x")]));
    assert_eq!(e.field, "edges[0].source");
}

#[test]
fn empty_figures_render() {
    allowed(&render(&bar_chart("", vec![])).unwrap());
    allowed(&render(&timeline("", vec![])).unwrap());
    allowed(&render(&graph("", vec![], vec![])).unwrap());
    allowed(&render(&bar_chart("zeros", vec![bar("a", 0), bar("b", 0)])).unwrap());
    allowed(&render(&bar_chart("huge", vec![bar("a", i64::MAX), bar("b", 1)])).unwrap());
}

/// A small deterministic generator (xorshift64*), so a failing case is found again from its seed.
struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A label of hostile pieces: markup, entities, quotes, handlers, `javascript:` and `href`,
    /// control characters, non-characters and text in other scripts.
    fn label(&mut self) -> String {
        const PIECES: &[&str] = &[
            "<",
            ">",
            "&",
            "\"",
            "'",
            "&amp;",
            "&#60;",
            "&lt;script&gt;",
            "<script>alert(1)</script>",
            "</text>",
            "<foreignObject>",
            "]]>",
            "<![CDATA[x]]>",
            "<!--",
            "-->",
            "<?xml version=\"1.0\"?>",
            "<!DOCTYPE svg>",
            "javascript:alert(1)",
            " onload=\"alert(1)\"",
            "onclick=",
            " href=\"https://example.org\"",
            "xlink:href=",
            "style=\"x\"",
            "url(#a)",
            "\u{0}",
            "\u{1}",
            "\u{8}",
            "\t",
            "\n",
            "\r",
            "\u{1b}",
            "\u{7f}",
            "\u{85}",
            "\u{9f}",
            "\u{FFFE}",
            "\u{FFFF}",
            "\u{202E}",
            "\u{FEFF}",
            "Zürich",
            "東京",
            "🦀",
            "plain",
            " ",
            "",
        ];
        (0..self.below(6))
            .map(|_| PIECES[self.below(PIECES.len())])
            .collect()
    }

    fn instant(&mut self) -> String {
        let zone = ["Z", "+02:00", "-11:30", "z"][self.below(4)];
        format!(
            "20{:02}-{:02}-{:02}T{:02}:{:02}:{:02}{zone}",
            self.below(100),
            1 + self.below(12),
            1 + self.below(28),
            self.below(24),
            self.below(60),
            self.below(60)
        )
    }

    fn figure(&mut self) -> ma::FigureSpec {
        let title = self.label();
        match self.below(3) {
            0 => {
                let bars = (0..self.below(12))
                    .map(|_| {
                        let count = [0, 1, 7, 1_000_000, i64::MAX][self.below(5)];
                        bar(&self.label(), count)
                    })
                    .collect();
                bar_chart(&title, bars)
            }
            1 => {
                let entries = (0..self.below(12))
                    .map(|_| entry(&self.instant(), &self.label()))
                    .collect();
                timeline(&title, entries)
            }
            _ => {
                let n = self.below(10);
                let nodes = (0..n)
                    .map(|i| node(&format!("{i}:{}", self.label()), &self.label()))
                    .collect::<Vec<_>>();
                let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
                let edges = if n == 0 {
                    vec![]
                } else {
                    (0..self.below(15))
                        .map(|_| edge(&ids[self.below(n)], &ids[self.below(n)], &self.label()))
                        .collect()
                };
                graph(&title, nodes, edges)
            }
        }
    }
}

/// The labels and title a figure draws, as drawn.
fn labels(spec: &ma::FigureSpec) -> Vec<String> {
    let mut out = Vec::new();
    match spec {
        ma::FigureSpec::BarChart(c) => {
            out.push(c.title.clone());
            out.extend(c.bars.iter().map(|b| b.label.clone()));
        }
        ma::FigureSpec::Timeline(t) => {
            out.push(t.title.clone());
            out.extend(t.entries.iter().map(|e| e.label.clone()));
        }
        ma::FigureSpec::RelationshipGraph(g) => {
            out.push(g.title.clone());
            out.extend(g.nodes.iter().map(|n| n.label.clone()));
            out.extend(g.edges.iter().map(|e| e.label.clone()));
        }
    }
    out.iter().map(|l| drawn(l)).collect()
}

#[test]
fn every_generated_figure_stays_inside_the_allow_list() {
    for seed in 1..=2_000u64 {
        let mut g = Gen(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let spec = g.figure();
        let svg = render(&spec).unwrap_or_else(|e| panic!("seed {seed}: {e:?} for {spec:?}"));
        assert_eq!(
            svg,
            render(&spec).unwrap(),
            "seed {seed}: the same figure gives the same bytes"
        );
        let texts = allowed(&svg);
        for label in labels(&spec) {
            assert!(
                texts.contains(&label),
                "seed {seed}: the label {label:?} is drawn as itself, in {texts:?}"
            );
        }
        let lower = svg.to_ascii_lowercase();
        for banned in ["<script", "<foreignobject", "<style", "<a ", "<!", "<?"] {
            assert!(!lower.contains(banned), "seed {seed}: {banned} in {svg}");
        }
    }
}

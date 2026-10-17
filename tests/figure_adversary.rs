//! Adversary cases for `story:figure-svg-renderer` (`cortex_cli::ask::figure`).
//!
//! Each case drives the renderer against what the story, `spec/domains/ask.yaml` and the ask
//! service page say it does. A case marked `#[ignore = "finding: …"]` fails today for the reason
//! it names; the others hold and pin a boundary the unit's own suite leaves open.

use std::panic::{catch_unwind, AssertUnwindSafe};

use cortex_cli::ask::figure::{render, FigureError, MAX_BYTES, MAX_NODES};
use cortex_model::ask as ma;
use cortex_model::primitives::Timestamp;

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

/// The texts of every `text` and `title` element, in document order, after a well-formedness parse.
fn texts(svg: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(svg).expect("a figure is well-formed XML");
    doc.descendants()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "text" | "title"))
        .map(|n| n.text().unwrap_or("").to_string())
        .collect()
}

/// `render(spec)`, with a panic turned into a failure that names it.
fn render_no_panic(spec: &ma::FigureSpec) -> Result<String, FigureError> {
    match catch_unwind(AssertUnwindSafe(|| render(spec))) {
        Ok(r) => r,
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            panic!("render panicked instead of refusing with a typed error: {msg}")
        }
    }
}

// ---- Timeline instants: the story's refusal is typed, never a panic -------------------------

#[test]
#[ignore = "finding: a timeline instant with a multibyte character at byte 18 panics in evidence::rfc3339_ms (&text[17..19]) instead of FigureInvalid"]
fn a_timeline_instant_with_a_multibyte_character_is_refused_not_a_panic() {
    // `é` takes bytes 18 and 19: bytes 10, 13 and 16 are the ASCII the parser checks, 19 is not a
    // character boundary.
    let spec = timeline("t", vec![entry("2026-01-01T00:00:0\u{e9}Z", "x")]);
    match render_no_panic(&spec) {
        Err(FigureError::Invalid(e)) => assert_eq!(e.field, "entries[0].at"),
        other => panic!("refused as invalid, got {other:?}"),
    }
}

#[test]
#[ignore = "finding: 'Z' followed by an offset ('…00Z01:00') is not RFC 3339 and is read as +01:00 instead of refused"]
fn an_instant_with_z_and_then_an_offset_is_refused() {
    let spec = timeline("t", vec![entry("2026-01-01T00:00:00Z01:00", "x")]);
    match render(&spec) {
        Err(FigureError::Invalid(e)) => assert_eq!(e.field, "entries[0].at"),
        Ok(svg) => panic!("drawn, with the instant read as {:?}", texts(&svg)),
        other => panic!("refused as invalid, got {other:?}"),
    }
}

#[test]
fn entries_at_one_instant_keep_the_order_given_across_offsets() {
    // Three spellings of one instant, then an earlier one last: the earlier one moves first, the
    // three keep their order (a stable sort; the unit's suite has no tie).
    let spec = timeline(
        "t",
        vec![
            entry("2026-01-01T01:00:00+01:00", "first"),
            entry("2026-01-01T00:00:00Z", "second"),
            entry("2025-12-31T23:00:00-01:00", "third"),
            entry("2025-06-01T00:00:00Z", "earliest"),
        ],
    );
    let t = texts(&render(&spec).unwrap());
    let labels: Vec<&str> = t
        .iter()
        .map(String::as_str)
        .filter(|s| ["first", "second", "third", "earliest"].contains(s))
        .collect();
    assert_eq!(labels, ["earliest", "first", "second", "third"]);
    assert!(t.iter().any(|s| s == "2026-01-01T00:00:00Z"), "{t:?}");
}

// ---- The byte cap: exactly at renders, one over is refused ---------------------------------

fn one_bar(label_len: usize) -> ma::FigureSpec {
    bar_chart("t", vec![bar(&"x".repeat(label_len), 0)])
}

#[test]
fn a_figure_of_exactly_the_byte_cap_renders_and_one_byte_more_is_refused() {
    let base = render(&one_bar(0)).unwrap().len();
    let at = MAX_BYTES - base;
    let svg = render(&one_bar(at)).expect("a figure of exactly the cap renders");
    assert_eq!(svg.len(), MAX_BYTES);
    match render(&one_bar(at + 1)) {
        Err(FigureError::TooLarge(e)) => {
            assert_eq!(
                (e.limit.as_str(), e.max, e.actual),
                ("bytes", MAX_BYTES as i64, MAX_BYTES as i64 + 1)
            );
        }
        other => panic!("refused as too large, got {:?}", other.map(|s| s.len())),
    }
}

#[test]
fn a_very_long_label_is_refused_whole() {
    let spec = graph("t", vec![node("a", &"y".repeat(4 << 20))], vec![]);
    match render(&spec) {
        Err(FigureError::TooLarge(e)) => {
            assert_eq!(e.limit, "bytes");
            assert!(e.actual > 4 << 20, "{}", e.actual);
        }
        other => panic!("refused as too large, got {:?}", other.map(|s| s.len())),
    }
}

// ---- Text that is legal XML but hostile to a reader ----------------------------------------

#[test]
fn separators_bidi_overrides_and_noncharacters_are_drawn_as_themselves() {
    let label = "a\u{2028}b\u{2029}c\u{202E}evil\u{202C}\u{2066}d\u{FEFF}\u{FDD0}\u{1FFFE}";
    let svg = render(&bar_chart(label, vec![bar(label, 1)])).unwrap();
    let t = texts(&svg);
    assert_eq!(t.iter().filter(|s| *s == label).count(), 3, "{t:?}");
}

#[test]
fn entity_and_cdata_lookalikes_never_reach_the_output_as_markup() {
    let label = "<!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]>&e;<![CDATA[<script>]]>";
    let svg = render(&graph(
        label,
        vec![node("n", label)],
        vec![edge("n", "n", label)],
    ))
    .unwrap();
    for banned in ["<!", "&e;", "<script", "]]>"] {
        assert!(!svg.contains(banned), "{banned} in {svg}");
    }
    assert_eq!(texts(&svg).iter().filter(|s| *s == label).count(), 4);
}

// ---- Graph geometry: finite, inside the view box, the same every time ----------------------

/// Every number in a geometry attribute and in a path, with the figure's width and height.
fn numbers(svg: &str) -> (f64, f64, Vec<(String, f64)>) {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let root = doc.root_element();
    let w: f64 = root.attribute("width").unwrap().parse().unwrap();
    let h: f64 = root.attribute("height").unwrap().parse().unwrap();
    let mut out = Vec::new();
    for n in doc.descendants().filter(|n| n.is_element()) {
        for a in n.attributes() {
            if matches!(
                a.name(),
                "x" | "y" | "x1" | "y1" | "x2" | "y2" | "cx" | "cy"
            ) {
                out.push((a.name().to_string(), a.value().parse().unwrap()));
            }
            if a.name() == "d" {
                let nums: Vec<f64> = a
                    .value()
                    .split(' ')
                    .filter(|t| !matches!(*t, "M" | "L" | "Z"))
                    .map(|t| t.parse().unwrap())
                    .collect();
                for p in nums.chunks(2) {
                    out.push(("x".into(), p[0]));
                    out.push(("y".into(), p[1]));
                }
            }
        }
    }
    (w, h, out)
}

#[test]
fn every_graph_size_draws_finite_coordinates_inside_its_view_box() {
    for n in 0..=MAX_NODES {
        let nodes: Vec<_> = (0..n).map(|i| node(&format!("n{i}"), "n")).collect();
        let edges: Vec<_> = (0..n)
            .flat_map(|i| {
                [
                    edge(&format!("n{i}"), &format!("n{}", (i + 1) % n), "e"),
                    edge(&format!("n{i}"), &format!("n{i}"), "self"),
                ]
            })
            .collect();
        let spec = graph("t", nodes, edges);
        let svg = render(&spec).unwrap();
        assert_eq!(svg, render(&spec.clone()).unwrap(), "n={n}: same bytes");
        for bad in ["NaN", "inf", "-0.0"] {
            assert!(!svg.contains(bad), "n={n}: {bad} in the figure");
        }
        let (w, h, nums) = numbers(&svg);
        for (name, v) in nums {
            let max = if name.starts_with('x') || name == "cx" {
                w
            } else {
                h
            };
            assert!(
                v.is_finite() && (0.0..=max).contains(&v),
                "n={n}: {name}={v} outside 0..={max}"
            );
        }
    }
}

#[test]
fn duplicate_and_reverse_edges_and_self_loops_render() {
    let spec = graph(
        "t",
        vec![node("a", "A"), node("b", "B")],
        vec![
            edge("a", "b", "1"),
            edge("a", "b", "2"),
            edge("b", "a", "3"),
            edge("a", "a", "4"),
        ],
    );
    let t = texts(&render(&spec).unwrap());
    for want in ["1", "2", "3", "4", "A", "B"] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
}

#[test]
fn a_duplicate_empty_node_id_is_refused() {
    match render(&graph("t", vec![node("", "a"), node("", "b")], vec![])) {
        Err(FigureError::Invalid(e)) => assert_eq!(e.field, "nodes[1].id"),
        other => panic!("refused as invalid, got {other:?}"),
    }
}

// ---- Bar extremes --------------------------------------------------------------------------

#[test]
fn bar_widths_are_proportional_and_a_nonzero_count_is_visible() {
    let svg = render(&bar_chart(
        "t",
        vec![
            bar("max", i64::MAX),
            bar("one", 1),
            bar("zero", 0),
            bar("half", i64::MAX / 2),
        ],
    ))
    .unwrap();
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let widths: Vec<&str> = doc
        .descendants()
        .filter(|n| n.has_tag_name("rect"))
        .map(|n| n.attribute("width").unwrap())
        .collect();
    assert_eq!(widths, ["360", "1", "0", "179"]);
}

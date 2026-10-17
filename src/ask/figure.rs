//! Figures drawn as SVG (`cortex.ask.FigureSpec`): a bar chart, a timeline or a relationship
//! graph, from typed values only, by cortex's own code.
//!
//! The output is built here, element by element, from a fixed set: the elements `svg`, `g`,
//! `rect`, `line`, `path`, `circle`, `text` and `title`, and geometry, fill, stroke and text
//! attributes whose values are numbers, colours and keywords this module writes. No title or
//! label ever reaches an attribute: each is the text of a `text` or `title` element, escaped
//! ([`text`]), with every control character (and U+FFFE, U+FFFF, which are no XML characters)
//! drawn as U+FFFD. So the output holds no script, no `foreignObject`, no `style`, no event
//! handler and no `href`, whatever a label says. Nothing stands between the elements, not even
//! whitespace: the only text of a figure is its titles and labels.
//!
//! The same figure gives the same bytes: the layout is integer arithmetic, except the graph's
//! circle, whose sines and cosines are computed here from IEEE operations alone ([`sin_cos`])
//! rather than the platform's maths library, and every coordinate is written to one decimal.
//!
//! A figure over a cap is refused whole ([`FigureError::TooLarge`]), never cut: the counts are
//! checked before anything is drawn, and the size of the whole rendering after.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use cortex_model::ask as ma;

/// The most bars a bar chart has.
pub const MAX_BARS: usize = 50;
/// The most entries a timeline has.
pub const MAX_ENTRIES: usize = 200;
/// The most nodes a relationship graph has.
pub const MAX_NODES: usize = 100;
/// The most edges a relationship graph has.
pub const MAX_EDGES: usize = 300;
/// The most bytes of SVG a figure is.
pub const MAX_BYTES: usize = 262_144;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const INK: &str = "#222222";
const BAR: &str = "#4c78a8";
const EDGE: &str = "#888888";
const NODE: &str = "#e45756";

/// Why a figure was not drawn. Nothing is drawn in either case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigureError {
    /// Over a cap (`cortex.ask.FigureTooLarge`).
    TooLarge(ma::FigureTooLarge),
    /// A value it cannot be drawn from (`cortex.ask.FigureInvalid`).
    Invalid(ma::FigureInvalid),
}

impl fmt::Display for FigureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FigureError::TooLarge(e) => write!(
                f,
                "the figure is too large: {} {} over the cap of {}",
                e.actual, e.limit, e.max
            ),
            FigureError::Invalid(e) => write!(f, "{}: {}", e.field, e.reason),
        }
    }
}

impl std::error::Error for FigureError {}

fn too_large(limit: &str, max: usize, actual: usize) -> FigureError {
    FigureError::TooLarge(ma::FigureTooLarge {
        limit: limit.into(),
        max: i64::try_from(max).unwrap_or(i64::MAX),
        actual: i64::try_from(actual).unwrap_or(i64::MAX),
    })
}

fn invalid(field: String, reason: &str) -> FigureError {
    FigureError::Invalid(ma::FigureInvalid {
        field,
        reason: reason.into(),
    })
}

fn cap(limit: &str, max: usize, actual: usize) -> Result<(), FigureError> {
    if actual > max {
        return Err(too_large(limit, max, actual));
    }
    Ok(())
}

/// `spec` as SVG, or why it is not drawn.
pub fn render(spec: &ma::FigureSpec) -> Result<String, FigureError> {
    let svg = match spec {
        ma::FigureSpec::BarChart(c) => bar_chart(c)?,
        ma::FigureSpec::Timeline(t) => timeline(t)?,
        ma::FigureSpec::RelationshipGraph(g) => graph(g)?,
    };
    cap("bytes", MAX_BYTES, svg.len())?;
    Ok(svg)
}

/// `s` as the text of an element: markup characters escaped, and each control character,
/// U+FFFE and U+FFFF as U+FFFD.
fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() || c == '\u{FFFE}' || c == '\u{FFFF}' => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

/// A coordinate to one decimal, never `-0.0`.
fn num(v: f64) -> String {
    let s = format!("{v:.1}");
    if s == "-0.0" {
        "0.0".into()
    } else {
        s
    }
}

/// The opening of a figure `width` by `height`, with its title as `title` and as a heading.
fn open(width: i64, height: i64, title: &str) -> String {
    let t = text(title);
    format!(
        "<svg xmlns=\"{SVG_NS}\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\" font-family=\"sans-serif\" font-size=\"12\" \
         fill=\"{INK}\"><title>{t}</title>\
         <text x=\"16\" y=\"28\" font-size=\"16\" font-weight=\"bold\">{t}</text>"
    )
}

const TOP: i64 = 48;

fn bar_chart(c: &ma::BarChart) -> Result<String, FigureError> {
    cap("bars", MAX_BARS, c.bars.len())?;
    for (i, b) in c.bars.iter().enumerate() {
        if b.count < 0 {
            return Err(invalid(format!("bars[{i}].count"), "a count is 0 or more"));
        }
    }
    const ROW: i64 = 24;
    const LABEL_END: i64 = 192;
    const BAR_X: i64 = 200;
    const BAR_MAX: i128 = 360;
    let n = c.bars.len() as i64;
    let mut s = open(640, TOP + n * ROW + 16, &c.title);
    let max = c.bars.iter().map(|b| b.count).max().unwrap_or(0);
    s.push_str("<g>");
    for (i, b) in c.bars.iter().enumerate() {
        let y = TOP + i as i64 * ROW;
        let mut w = if max > 0 {
            (i128::from(b.count) * BAR_MAX / i128::from(max)) as i64
        } else {
            0
        };
        if b.count > 0 && w == 0 {
            w = 1;
        }
        let _ = write!(
            s,
            "<text x=\"{LABEL_END}\" y=\"{}\" text-anchor=\"end\">{}</text>\
             <rect x=\"{BAR_X}\" y=\"{}\" width=\"{w}\" height=\"16\" fill=\"{BAR}\"/>\
             <text x=\"{}\" y=\"{}\">{}</text>",
            y + 16,
            text(&b.label),
            y + 4,
            BAR_X + w + 6,
            y + 16,
            b.count
        );
    }
    s.push_str("</g></svg>");
    Ok(s)
}

fn timeline(t: &ma::Timeline) -> Result<String, FigureError> {
    cap("entries", MAX_ENTRIES, t.entries.len())?;
    let mut order = Vec::with_capacity(t.entries.len());
    for (i, e) in t.entries.iter().enumerate() {
        let Some(ms) = crate::evidence::rfc3339_ms(&e.at.0) else {
            return Err(invalid(
                format!("entries[{i}].at"),
                "an instant is RFC 3339, such as 2026-01-01T00:00:00Z",
            ));
        };
        order.push((ms, i));
    }
    // A stable sort: entries at one instant keep the order given.
    order.sort_by_key(|&(ms, _)| ms);
    const ROW: i64 = 28;
    const AXIS_X: i64 = 24;
    let n = t.entries.len() as i64;
    let mut s = open(720, TOP + n * ROW + 16, &t.title);
    if n > 0 {
        let _ = write!(
            s,
            "<line x1=\"{AXIS_X}\" y1=\"{TOP}\" x2=\"{AXIS_X}\" y2=\"{}\" stroke=\"{EDGE}\" \
             stroke-width=\"2\"/>",
            TOP + n * ROW
        );
    }
    s.push_str("<g>");
    for (row, &(ms, i)) in order.iter().enumerate() {
        let cy = TOP + row as i64 * ROW + 14;
        let _ = write!(
            s,
            "<circle cx=\"{AXIS_X}\" cy=\"{cy}\" r=\"5\" fill=\"{BAR}\"/>\
             <text x=\"40\" y=\"{}\">{}</text>\
             <text x=\"200\" y=\"{}\">{}</text>",
            cy + 4,
            text(&crate::sources::rfc3339(ms)),
            cy + 4,
            text(&t.entries[i].label)
        );
    }
    s.push_str("</g></svg>");
    Ok(s)
}

/// The sine and cosine of `x`, for `x` in [-π, π], by their Taylor series in IEEE operations
/// only, so every platform computes the same bits.
fn sin_cos(x: f64) -> (f64, f64) {
    let (mut sin, mut cos) = (0.0, 0.0);
    let (mut s_term, mut c_term) = (x, 1.0);
    let x2 = x * x;
    for k in 0..30 {
        sin += s_term;
        cos += c_term;
        let k = f64::from(k);
        s_term = -s_term * x2 / ((2.0 * k + 2.0) * (2.0 * k + 3.0));
        c_term = -c_term * x2 / ((2.0 * k + 1.0) * (2.0 * k + 2.0));
    }
    (sin, cos)
}

fn graph(g: &ma::RelationshipGraph) -> Result<String, FigureError> {
    cap("nodes", MAX_NODES, g.nodes.len())?;
    cap("edges", MAX_EDGES, g.edges.len())?;
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in g.nodes.iter().enumerate() {
        if index.insert(n.id.as_str(), i).is_some() {
            return Err(invalid(
                format!("nodes[{i}].id"),
                "a node id is given once in a graph",
            ));
        }
    }
    let mut ends = Vec::with_capacity(g.edges.len());
    for (i, e) in g.edges.iter().enumerate() {
        let Some(&from) = index.get(e.source.as_str()) else {
            return Err(invalid(
                format!("edges[{i}].source"),
                "an edge's source is the id of a node of the graph",
            ));
        };
        let Some(&to) = index.get(e.target.as_str()) else {
            return Err(invalid(
                format!("edges[{i}].target"),
                "an edge's target is the id of a node of the graph",
            ));
        };
        ends.push((from, to));
    }

    let n = g.nodes.len();
    let radius = (n as f64 * 10.0).max(160.0);
    let margin = 180.0;
    let size = (2.0 * (radius + margin)) as i64;
    let centre = radius + margin;
    let top = TOP as f64;
    let at: Vec<(f64, f64, f64, f64)> = (0..n)
        .map(|i| {
            if n == 1 {
                return (centre, centre + top, 0.0, -1.0);
            }
            let mut a =
                2.0 * std::f64::consts::PI * i as f64 / n as f64 - std::f64::consts::FRAC_PI_2;
            if a > std::f64::consts::PI {
                a -= 2.0 * std::f64::consts::PI;
            }
            let (sin, cos) = sin_cos(a);
            (centre + radius * cos, centre + top + radius * sin, cos, sin)
        })
        .collect();

    let mut s = open(size, size + TOP, &g.title);
    let _ = write!(s, "<g stroke=\"{EDGE}\" stroke-width=\"1\">");
    for &(from, to) in &ends {
        let (x1, y1, ..) = at[from];
        let (x2, y2, ..) = at[to];
        if from == to {
            let _ = write!(
                s,
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\"/>",
                num(x1),
                num(y1 - 14.0)
            );
            continue;
        }
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len = (dx * dx + dy * dy).sqrt();
        let _ = write!(
            s,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            num(x1),
            num(y1),
            num(x2),
            num(y2)
        );
        if len > 16.0 {
            let (ux, uy) = (dx / len, dy / len);
            let (tx, ty) = (x2 - ux * 8.0, y2 - uy * 8.0);
            let (bx, by) = (tx - ux * 8.0, ty - uy * 8.0);
            let _ = write!(
                s,
                "<path d=\"M {} {} L {} {} L {} {} Z\" fill=\"{EDGE}\"/>",
                num(tx),
                num(ty),
                num(bx - uy * 4.0),
                num(by + ux * 4.0),
                num(bx + uy * 4.0),
                num(by - ux * 4.0)
            );
        }
    }
    s.push_str("</g><g font-size=\"10\" text-anchor=\"middle\">");
    for (e, &(from, to)) in g.edges.iter().zip(&ends) {
        let (x1, y1, ..) = at[from];
        let (x2, y2, ..) = at[to];
        let (x, y) = if from == to {
            (x1, y1 - 28.0)
        } else {
            // 40 % of the way from the source: two edges crossing at their midpoints, or an edge
            // and its reverse, keep their labels apart.
            (x1 + (x2 - x1) * 0.4, y1 + (y2 - y1) * 0.4 - 3.0)
        };
        let _ = write!(
            s,
            "<text x=\"{}\" y=\"{}\">{}</text>",
            num(x),
            num(y),
            text(&e.label)
        );
    }
    s.push_str("</g><g>");
    for (node, &(x, y, cos, sin)) in g.nodes.iter().zip(&at) {
        let anchor = if cos > 0.01 {
            "start"
        } else if cos < -0.01 {
            "end"
        } else {
            "middle"
        };
        let _ = write!(
            s,
            "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"{NODE}\"/>\
             <text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            num(x),
            num(y),
            num(x + cos * 12.0),
            num(y + sin * 12.0 + 4.0),
            text(&node.label)
        );
    }
    s.push_str("</g></svg>");
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_and_cosine_agree_with_the_platform_to_display_precision() {
        for i in 0..=360 {
            let a = -std::f64::consts::PI + 2.0 * std::f64::consts::PI * f64::from(i) / 360.0;
            let (s, c) = sin_cos(a);
            assert!((s - a.sin()).abs() < 1e-12, "{a}");
            assert!((c - a.cos()).abs() < 1e-12, "{a}");
        }
    }

    #[test]
    fn markup_characters_are_escaped_and_controls_replaced() {
        assert_eq!(
            text("<a href='x'>&\"\u{0}\u{FFFF}"),
            "&lt;a href=&apos;x&apos;&gt;&amp;&quot;\u{FFFD}\u{FFFD}"
        );
    }

    #[test]
    fn a_coordinate_is_never_negative_zero() {
        assert_eq!(num(-0.04), "0.0");
        assert_eq!(num(-0.05), "-0.1");
    }
}

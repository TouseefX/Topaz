

use std::collections::HashMap;

use cfg::{CfgNode, CfgSnapshot, EdgeKind};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

const ROW_HEIGHT: f32 = 160.0;
const COL_WIDTH: f32 = 280.0;
const NODE_WIDTH: f32 = 240.0;
const NODE_PADDING: f32 = 8.0;
const FONT_SIZE: f32 = 11.0;
const LINE_HEIGHT: f32 = 14.0;
const MAX_LABEL_LINES: usize = 10;

pub struct CfgViewState {
    pub zoom: f32,
    pub pan: Vec2,
    pub selected: Option<usize>,
    pub search_query: String,
    layout: Option<Layout>,

    auto_fit_pending: bool,
}

impl Default for CfgViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            selected: None,
            search_query: String::new(),
            layout: None,
            auto_fit_pending: false,
        }
    }
}

impl CfgViewState {

    pub fn reset(&mut self) {
        self.layout = None;
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
        self.selected = None;
        self.search_query.clear();
        self.auto_fit_pending = false;
    }
}

#[derive(Clone)]
struct LaidOutNode {
    id: usize,
    rect: Rect,
    truncated_label: String,
    is_entry: bool,
    statement_count: usize,
}

#[derive(Clone)]
struct LaidOutEdge {
    from: usize,
    to: usize,
    kind: EdgeKind,
}

#[derive(Clone)]
struct Layout {
    fingerprint: (String, usize, usize),
    nodes: Vec<LaidOutNode>,
    edges: Vec<LaidOutEdge>,
    node_by_id: HashMap<usize, usize>,
    bounds: Rect,
}

impl Layout {
    fn build(snapshot: &CfgSnapshot) -> Self {
        let fingerprint = (
            snapshot.name.clone(),
            snapshot.nodes.len(),
            snapshot.edges.len(),
        );

        let mut successors: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut predecessors: HashMap<usize, Vec<usize>> = HashMap::new();
        for e in &snapshot.edges {
            successors.entry(e.from).or_default().push(e.to);
            predecessors.entry(e.to).or_default().push(e.from);
        }

        let entry = snapshot.entry.unwrap_or_else(|| {
            snapshot.nodes.first().map(|n| n.id).unwrap_or(0)
        });
        let mut layer: HashMap<usize, usize> = HashMap::new();
        layer.insert(entry, 0);

        let max_iter = snapshot.nodes.len().max(1) * 4;
        for _ in 0..max_iter {
            let mut changed = false;
            for e in &snapshot.edges {
                if let Some(&src) = layer.get(&e.from) {
                    let candidate = src + 1;
                    let entry = layer.entry(e.to).or_insert(candidate);
                    if *entry < candidate {

                        if candidate <= max_iter {
                            *entry = candidate;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        for n in &snapshot.nodes {
            layer.entry(n.id).or_insert(0);
        }

        let mut layers: Vec<Vec<&CfgNode>> = Vec::new();
        for n in &snapshot.nodes {
            let l = *layer.get(&n.id).unwrap_or(&0);
            if layers.len() <= l {
                layers.resize_with(l + 1, Vec::new);
            }
            layers[l].push(n);
        }

        for _sweep in 0..4 {
            for li in 1..layers.len() {
                let prev_positions: HashMap<usize, f32> = layers[li - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.id, i as f32))
                    .collect();
                layers[li].sort_by(|a, b| {
                    let av = barycenter(&predecessors, &prev_positions, a.id);
                    let bv = barycenter(&predecessors, &prev_positions, b.id);
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            for li in (0..layers.len().saturating_sub(1)).rev() {
                let next_positions: HashMap<usize, f32> = layers[li + 1]
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.id, i as f32))
                    .collect();
                layers[li].sort_by(|a, b| {
                    let av = barycenter(&successors, &next_positions, a.id);
                    let bv = barycenter(&successors, &next_positions, b.id);
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        let mut nodes: Vec<LaidOutNode> = Vec::with_capacity(snapshot.nodes.len());
        let mut node_by_id: HashMap<usize, usize> = HashMap::new();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for (li, row) in layers.iter().enumerate() {
            let row_width = row.len() as f32 * COL_WIDTH;
            let start_x = -row_width / 2.0;
            for (ci, node) in row.iter().enumerate() {
                let (truncated, line_count) = truncate_label(&node.label);
                let height =
                    NODE_PADDING * 2.0 + LINE_HEIGHT * (line_count.max(1) as f32) + 16.0;
                let x = start_x + (ci as f32 + 0.5) * COL_WIDTH;
                let y = li as f32 * ROW_HEIGHT;
                let rect = Rect::from_min_size(
                    Pos2::new(x - NODE_WIDTH / 2.0, y),
                    Vec2::new(NODE_WIDTH, height),
                );
                min_x = min_x.min(rect.min.x);
                min_y = min_y.min(rect.min.y);
                max_x = max_x.max(rect.max.x);
                max_y = max_y.max(rect.max.y);
                node_by_id.insert(node.id, nodes.len());
                nodes.push(LaidOutNode {
                    id: node.id,
                    rect,
                    truncated_label: truncated,
                    is_entry: node.is_entry,
                    statement_count: node.statement_count,
                });
            }
        }

        let edges = snapshot
            .edges
            .iter()
            .map(|e| LaidOutEdge {
                from: e.from,
                to: e.to,
                kind: e.kind,
            })
            .collect();

        let bounds = if nodes.is_empty() {
            Rect::from_center_size(Pos2::ZERO, Vec2::splat(100.0))
        } else {
            Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
        };

        Self {
            fingerprint,
            nodes,
            edges,
            node_by_id,
            bounds,
        }
    }
}

fn barycenter(
    neighbors: &HashMap<usize, Vec<usize>>,
    positions: &HashMap<usize, f32>,
    node_id: usize,
) -> f32 {
    let Some(ns) = neighbors.get(&node_id) else {
        return f32::INFINITY;
    };
    let mut sum = 0.0;
    let mut count = 0.0;
    for n in ns {
        if let Some(&p) = positions.get(n) {
            sum += p;
            count += 1.0;
        }
    }
    if count == 0.0 { f32::INFINITY } else { sum / count }
}

fn truncate_label(s: &str) -> (String, usize) {
    let mut out = String::new();
    let mut count = 0;
    for line in s.lines().take(MAX_LABEL_LINES) {
        if count > 0 {
            out.push('\n');
        }

        if line.len() > 40 {
            out.push_str(&line[..37]);
            out.push_str("…");
        } else {
            out.push_str(line);
        }
        count += 1;
    }
    let total_lines = s.lines().count();
    if total_lines > MAX_LABEL_LINES {
        out.push_str(&format!("\n… (+{} more)", total_lines - MAX_LABEL_LINES));
        count += 1;
    }
    (out, count)
}

pub fn show(ui: &mut egui::Ui, state: &mut CfgViewState, snapshot: &CfgSnapshot) {
    let fingerprint = (
        snapshot.name.clone(),
        snapshot.nodes.len(),
        snapshot.edges.len(),
    );
    if state.layout.as_ref().map(|l| &l.fingerprint) != Some(&fingerprint) {
        state.layout = Some(Layout::build(snapshot));
        state.pan = Vec2::ZERO;
        state.zoom = 1.0;
        state.selected = None;
        state.auto_fit_pending = true;
    }
    let mut do_fit = false;
    let mut do_reset = false;
    let mut do_export = false;
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(&snapshot.name).strong().monospace());
        ui.weak(format!(
            "· {} blocks · {} edges",
            snapshot.nodes.len(),
            snapshot.edges.len()
        ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            if ui.small_button("Export DOT…").clicked() {
                do_export = true;
            }
            ui.add_space(2.0);
            if ui.small_button("100%").clicked() {
                do_reset = true;
            }
            if ui.small_button("Fit").clicked() {
                do_fit = true;
            }
            ui.add_space(6.0);
            ui.weak(format!("zoom {:.0}%", state.zoom * 100.0));
        });
    });

    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label("Find block:");
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .desired_width(220.0)
                .hint_text("statement substring…"),
        );
        if !state.search_query.is_empty() {
            let q = state.search_query.to_lowercase();
            let matches = snapshot
                .nodes
                .iter()
                .filter(|n| n.label.to_lowercase().contains(&q))
                .count();
            ui.weak(format!(
                "{matches} match{}",
                if matches == 1 { "" } else { "es" }
            ));
            if ui.small_button("Clear").clicked() {
                state.search_query.clear();
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            ui.weak("drag = pan · scroll = zoom · click = inspect");
        });
    });

    ui.add_space(2.0);
    ui.separator();

    let available_for_fit = ui.available_size();
    if do_fit || state.auto_fit_pending {
        fit_to_view(state, available_for_fit);
        state.auto_fit_pending = false;
    }
    if do_reset {
        state.zoom = 1.0;
        state.pan = Vec2::ZERO;
    }
    if do_export {
        export_dot(snapshot);
    }

    let layout = state.layout.as_ref().unwrap();

    let available = ui.available_size_before_wrap();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    let viewport = response.rect;
    let painter_clip = painter.with_clip_rect(viewport);

    painter_clip.rect_filled(viewport, 0.0, ui.visuals().extreme_bg_color);

    if response.dragged() {
        state.pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let zoom_factor = (scroll * 0.0015).exp();

            let pointer = ui.input(|i| i.pointer.hover_pos()).unwrap_or(viewport.center());
            let center = viewport.center();
            let offset_before = (pointer - center) - state.pan;
            state.zoom = (state.zoom * zoom_factor).clamp(0.2, 4.0);
            let offset_after = offset_before * zoom_factor;
            state.pan = (pointer - center) - offset_after;
        }
    }

    let click_pos = if response.clicked() {
        response.interact_pointer_pos()
    } else {
        None
    };

    let center = viewport.center();
    let to_screen = |p: Pos2| -> Pos2 {
        Pos2::new(
            center.x + state.pan.x + p.x * state.zoom,
            center.y + state.pan.y + p.y * state.zoom,
        )
    };

    let search_lower = state.search_query.to_lowercase();
    let has_search = !search_lower.is_empty();

    for edge in &layout.edges {
        let (Some(&fi), Some(&ti)) = (
            layout.node_by_id.get(&edge.from),
            layout.node_by_id.get(&edge.to),
        ) else {
            continue;
        };
        let from = &layout.nodes[fi];
        let to = &layout.nodes[ti];
        let from_below = from.rect.center().y < to.rect.center().y;

        let (start_world, end_world) = if from_below {
            (
                Pos2::new(from.rect.center().x, from.rect.max.y),
                Pos2::new(to.rect.center().x, to.rect.min.y),
            )
        } else {

            (
                Pos2::new(from.rect.max.x, from.rect.center().y),
                Pos2::new(to.rect.max.x, to.rect.center().y),
            )
        };
        let start = to_screen(start_world);
        let end = to_screen(end_world);

        let color = match edge.kind {
            EdgeKind::Unconditional => Color32::from_rgb(150, 150, 150),
            EdgeKind::Then => Color32::from_rgb(80, 200, 110),
            EdgeKind::Else => Color32::from_rgb(220, 100, 100),
        };
        let stroke = Stroke::new(1.5, color);
        let mid = draw_bezier_edge(&painter_clip, start, end, stroke, from_below);

        let label = match edge.kind {
            EdgeKind::Unconditional => "",
            EdgeKind::Then => "T",
            EdgeKind::Else => "F",
        };
        if !label.is_empty() {
            painter_clip.text(
                mid,
                Align2::CENTER_CENTER,
                label,
                FontId::monospace(10.0 * state.zoom.clamp(0.6, 1.4)),
                color,
            );
        }
    }

    let mut clicked_node: Option<usize> = None;
    for node in &layout.nodes {
        let min = to_screen(node.rect.min);
        let max = to_screen(node.rect.max);
        let screen_rect = Rect::from_min_max(min, max);

        if !viewport.intersects(screen_rect) {
            continue;
        }

        let selected = state.selected == Some(node.id);
        let is_hovered = click_pos.map(|p| screen_rect.contains(p)).unwrap_or(false);
        if is_hovered {
            clicked_node = Some(node.id);
        }

        let is_match = has_search
            && snapshot
                .nodes
                .iter()
                .find(|n| n.id == node.id)
                .map(|n| n.label.to_lowercase().contains(&search_lower))
                .unwrap_or(false);

        let bg = if selected {
            Color32::from_rgb(60, 80, 110)
        } else if is_match {
            Color32::from_rgb(80, 60, 30)
        } else if node.is_entry {
            Color32::from_rgb(40, 70, 50)
        } else {
            Color32::from_rgb(40, 42, 50)
        };
        let border = if selected {
            Color32::from_rgb(120, 170, 230)
        } else if is_match {
            Color32::from_rgb(230, 180, 80)
        } else if node.is_entry {
            Color32::from_rgb(110, 200, 130)
        } else {
            Color32::from_rgb(80, 82, 92)
        };

        painter_clip.rect(
            screen_rect,
            6.0,
            bg,
            Stroke::new(1.5, border),
            StrokeKind::Outside,
        );

        let header_h = 16.0 * state.zoom.clamp(0.8, 1.4);
        let header_rect = Rect::from_min_size(
            screen_rect.min,
            Vec2::new(screen_rect.width(), header_h),
        );
        let header_text = if node.is_entry {
            format!("entry · #{} · {} stmt", node.id, node.statement_count)
        } else {
            format!("#{} · {} stmt", node.id, node.statement_count)
        };
        painter_clip.text(
            header_rect.left_center() + Vec2::new(6.0, 0.0),
            Align2::LEFT_CENTER,
            header_text,
            FontId::proportional(10.0 * state.zoom.clamp(0.7, 1.3)),
            Color32::from_rgb(200, 200, 210),
        );

        let body_rect = screen_rect.shrink2(Vec2::new(6.0, 4.0)).translate(Vec2::new(0.0, header_h));
        painter_clip.text(
            body_rect.left_top(),
            Align2::LEFT_TOP,
            &node.truncated_label,
            FontId::monospace(FONT_SIZE * state.zoom.clamp(0.6, 1.6)),
            Color32::from_rgb(220, 220, 220),
        );
    }

    if let Some(id) = clicked_node {
        state.selected = Some(id);
    }

    if layout.nodes.is_empty() {
        painter_clip.text(
            viewport.center(),
            Align2::CENTER_CENTER,
            "Empty CFG.",
            FontId::proportional(13.0),
            ui.visuals().weak_text_color(),
        );
    }
}

pub fn show_selected_panel(ui: &mut egui::Ui, state: &CfgViewState, snapshot: &CfgSnapshot) {
    if let Some(sel) = state.selected {
        if let Some(node) = snapshot.nodes.iter().find(|n| n.id == sel) {
            ui.label(
                egui::RichText::new(format!(
                    "Block #{}{}",
                    node.id,
                    if node.is_entry { " (entry)" } else { "" }
                ))
                .strong(),
            );
            ui.weak(format!("{} statements", node.statement_count));
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut node.label.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                );
            });
            return;
        }
    }
    ui.weak("Click a block to inspect its statements.");
}

fn draw_bezier_edge(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
    forward: bool,
) -> Pos2 {

    let (c1, c2) = if forward {
        let dy = (to.y - from.y).abs().max(40.0);
        let off = (dy * 0.45).min(120.0);
        (
            Pos2::new(from.x, from.y + off),
            Pos2::new(to.x, to.y - off),
        )
    } else {

        let dx = 80.0;
        (
            Pos2::new(from.x + dx, from.y),
            Pos2::new(to.x + dx, to.y),
        )
    };

    const SAMPLES: usize = 24;
    let mut prev = from;
    let mut mid = from;
    for i in 1..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let p = bezier_cubic(from, c1, c2, to, t);
        painter.line_segment([prev, p], stroke);
        if i == SAMPLES / 2 {
            mid = p;
        }
        prev = p;
    }

    let tangent = (to - bezier_cubic(from, c1, c2, to, 0.92)).normalized();
    let normal = Vec2::new(-tangent.y, tangent.x);
    let head = to - tangent * 12.0;
    let p1 = head + normal * 5.0;
    let p2 = head - normal * 5.0;
    painter.add(egui::Shape::convex_polygon(
        vec![to, p1, p2],
        stroke.color,
        Stroke::NONE,
    ));

    mid
}

fn bezier_cubic(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let b0 = u * u * u;
    let b1 = 3.0 * u * u * t;
    let b2 = 3.0 * u * t * t;
    let b3 = t * t * t;
    Pos2::new(
        b0 * p0.x + b1 * p1.x + b2 * p2.x + b3 * p3.x,
        b0 * p0.y + b1 * p1.y + b2 * p2.y + b3 * p3.y,
    )
}

#[cfg(not(target_os = "android"))]
fn export_dot(snapshot: &CfgSnapshot) {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "digraph cfg {{");
    let _ = writeln!(out, "  graph [rankdir=TB, bgcolor=\"#1a1a1a\", fontcolor=\"#dddddd\"];");
    let _ = writeln!(out, "  node  [shape=box, style=\"rounded,filled\", fillcolor=\"#2b2b2b\", color=\"#888\", fontcolor=\"#ddd\", fontname=\"Menlo\", fontsize=10];");
    let _ = writeln!(out, "  edge  [color=\"#999\", fontcolor=\"#999\"];");
    for n in &snapshot.nodes {
        let escaped = escape_dot(&n.label);
        let attrs = if n.is_entry {
            ", fillcolor=\"#2d4030\", color=\"#6cc870\""
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  N{} [label=\"#{}\\l{}\"{}];",
            n.id, n.id, escaped, attrs
        );
    }
    for e in &snapshot.edges {
        let (label, color) = match e.kind {
            EdgeKind::Unconditional => ("", "#999999"),
            EdgeKind::Then => ("T", "#50c870"),
            EdgeKind::Else => ("F", "#dc6464"),
        };
        let _ = writeln!(
            out,
            "  N{} -> N{} [label=\"{}\", color=\"{}\"];",
            e.from, e.to, label, color
        );
    }
    let _ = writeln!(out, "}}");

    let default_name = format!("{}.dot", sanitize_filename(&snapshot.name));
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export CFG as GraphViz DOT")
        .add_filter("GraphViz", &["dot", "gv"])
        .set_file_name(&default_name)
        .save_file()
    {
        let _ = std::fs::write(path, out);
    }
}

#[cfg(target_os = "android")]
fn export_dot(snapshot: &CfgSnapshot) {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "digraph cfg {{");
    let _ = writeln!(out, "  graph [rankdir=TB, bgcolor=\"#1a1a1a\", fontcolor=\"#dddddd\"];");
    let _ = writeln!(out, "  node  [shape=box, style=\"rounded,filled\", fillcolor=\"#2b2b2b\", color=\"#888\", fontcolor=\"#ddd\", fontname=\"Menlo\", fontsize=10];");
    let _ = writeln!(out, "  edge  [color=\"#999\", fontcolor=\"#999\"];");
    for n in &snapshot.nodes {
        let escaped = escape_dot(&n.label);
        let attrs = if n.is_entry {
            ", fillcolor=\"#2d4030\", color=\"#6cc870\""
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  N{} [label=\"#{}\\l{}\"{}];",
            n.id, n.id, escaped, attrs
        );
    }
    for e in &snapshot.edges {
        let (label, color) = match e.kind {
            EdgeKind::Unconditional => ("", "#999999"),
            EdgeKind::Then => ("T", "#50c870"),
            EdgeKind::Else => ("F", "#dc6464"),
        };
        let _ = writeln!(
            out,
            "  N{} -> N{} [label=\"{}\", color=\"{}\"];",
            e.from, e.to, label, color
        );
    }
    let _ = writeln!(out, "}}");

    // On Android, save to fixed location in Download folder
    let candidates = [
        format!("/sdcard/Download/{}.dot", sanitize_filename(&snapshot.name)),
        format!(
            "/storage/emulated/0/Download/{}.dot",
            sanitize_filename(&snapshot.name)
        ),
        format!("/data/data/com.touseefx.topaz/files/{}.dot", sanitize_filename(&snapshot.name)),
    ];
    for path in candidates {
        if std::fs::write(&path, &out).is_ok() {
            log::info!("DOT exported to {path}");
            return;
        }
    }
    // fallback to temp
    let mut p = std::env::temp_dir();
    p.push(format!("{}.dot", sanitize_filename(&snapshot.name)));
    let _ = std::fs::write(p, out);
}

fn escape_dot(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\l"),
            _ => out.push(c),
        }
    }
    out
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn fit_to_view(state: &mut CfgViewState, available: Vec2) {
    let Some(layout) = &state.layout else { return };
    if layout.nodes.is_empty() {
        state.zoom = 1.0;
        state.pan = Vec2::ZERO;
        return;
    }
    let bw = (layout.bounds.width() + 80.0).max(1.0);
    let bh = (layout.bounds.height() + 80.0).max(1.0);
    let zx = available.x / bw;
    let zy = available.y / bh;
    state.zoom = zx.min(zy).clamp(0.2, 4.0);

    let bcenter = layout.bounds.center().to_vec2();
    state.pan = -bcenter * state.zoom;
}

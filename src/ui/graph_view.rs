use crate::config::AppConfig;
use crate::filesystem;
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerMotion, EventControllerScroll, GestureClick, GestureDrag};
use rand::Rng;
use std::cell::RefCell;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Interactive Graph View (Force-Directed)
// ═══════════════════════════════════════════════
//
// Obsidian-style node graph that visualises files/folders as connected nodes.
//
// Features:
//   • Force-directed physics (repulsion + spring attraction)
//   • Mouse zoom (scroll) and pan (drag background)
//   • Node dragging
//   • Click on folder node → expand children
//   • Smooth 60 fps animation via glib tick callback

// ─── Data Structures ───

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: usize,
    pub label: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub parent_id: Option<usize>,
    // Physics state
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    // Visual
    pub radius: f64,
    pub color: NodeColor,
}

#[derive(Clone, Debug)]
pub struct NodeColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug)]
pub struct GraphState {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub next_id: usize,
    // Camera state
    pub cam_x: f64,
    pub cam_y: f64,
    pub zoom: f64,
    // Interaction
    pub dragged_node: Option<usize>,
    pub drag_offset_x: f64,
    pub drag_offset_y: f64,
    pub is_panning: bool,
    pub pan_start_x: f64,
    pub pan_start_y: f64,
    pub cam_start_x: f64,
    pub cam_start_y: f64,
    pub hovered_node: Option<usize>,
    // Physics toggle
    pub physics_enabled: bool,
}

impl GraphState {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            dragged_node: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            is_panning: false,
            pan_start_x: 0.0,
            pan_start_y: 0.0,
            cam_start_x: 0.0,
            cam_start_y: 0.0,
            hovered_node: None,
            physics_enabled: true,
        }
    }

    /// Adds a root node at the centre.
    fn add_root(&mut self, path: &PathBuf) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        self.nodes.push(GraphNode {
            id,
            label,
            path: path.clone(),
            is_dir: true,
            is_expanded: false,
            parent_id: None,
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            radius: 28.0,
            color: dir_color(),
        });
        id
    }

    /// Expands a directory node: adds children and edges.
    fn expand_node(&mut self, node_id: usize) {
        // Look up by ID, not by index — IDs are not array indices
        let node = match self.nodes.iter().find(|n| n.id == node_id) {
            Some(n) => n,
            None => return,
        };
        if !node.is_dir || node.is_expanded {
            return;
        }

        let path = node.path.clone();
        let parent_x = node.x;
        let parent_y = node.y;

        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node_id) {
            n.is_expanded = true;
        }

        let entries = filesystem::list_directory(&path, false);
        let count = entries.len();
        let mut rng = rand::thread_rng();

        for (i, entry) in entries.iter().enumerate() {
            let child_id = self.next_id;
            self.next_id += 1;

            // Place children in a circle around the parent
            let angle = if count > 0 {
                (i as f64 / count as f64) * 2.0 * PI
            } else {
                0.0
            };
            let dist = 120.0 + rng.gen_range(-20.0..20.0);
            let cx = parent_x + angle.cos() * dist;
            let cy = parent_y + angle.sin() * dist;

            let radius = if entry.is_dir { 22.0 } else { 14.0 };
            let color = if entry.is_dir {
                dir_color()
            } else {
                file_color_for_ext(&entry.extension)
            };

            self.nodes.push(GraphNode {
                id: child_id,
                label: entry.name.clone(),
                path: entry.path.clone(),
                is_dir: entry.is_dir,
                is_expanded: false,
                parent_id: Some(node_id),
                x: cx,
                y: cy,
                vx: 0.0,
                vy: 0.0,
                radius,
                color,
            });

            self.edges.push(GraphEdge {
                from: node_id,
                to: child_id,
            });
        }
    }

    /// Collapse a node: remove all descendants and their edges.
    fn collapse_node(&mut self, node_id: usize) {
        let is_expanded = self
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|n| n.is_expanded)
            .unwrap_or(false);
        if !is_expanded {
            return;
        }
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node_id) {
            n.is_expanded = false;
        }

        // Collect all descendant IDs
        let mut to_remove = Vec::new();
        let mut stack = vec![node_id];
        while let Some(nid) = stack.pop() {
            for node in &self.nodes {
                if node.parent_id == Some(nid) && node.id != node_id {
                    to_remove.push(node.id);
                    if node.is_dir {
                        stack.push(node.id);
                    }
                }
            }
        }

        self.nodes.retain(|n| !to_remove.contains(&n.id));
        self.edges
            .retain(|e| !to_remove.contains(&e.from) && !to_remove.contains(&e.to));
    }

    /// One step of the force-directed physics simulation.
    fn physics_step(&mut self) {
        if !self.physics_enabled {
            return;
        }

        let n = self.nodes.len();
        if n == 0 {
            return;
        }

        let repulsion = 8000.0;
        let spring_k = 0.02;
        let spring_rest = 120.0;
        let damping = 0.85;
        let max_speed = 8.0;

        // Accumulate forces
        let mut fx = vec![0.0f64; n];
        let mut fy = vec![0.0f64; n];

        // Repulsion between all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.nodes[i].x - self.nodes[j].x;
                let dy = self.nodes[i].y - self.nodes[j].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let force = repulsion / (dist * dist);
                let fdx = (dx / dist) * force;
                let fdy = (dy / dist) * force;
                fx[i] += fdx;
                fy[i] += fdy;
                fx[j] -= fdx;
                fy[j] -= fdy;
            }
        }

        // Spring attraction along edges
        for edge in &self.edges {
            let fi = self.nodes.iter().position(|n| n.id == edge.from);
            let ti = self.nodes.iter().position(|n| n.id == edge.to);
            if let (Some(fi), Some(ti)) = (fi, ti) {
                let dx = self.nodes[ti].x - self.nodes[fi].x;
                let dy = self.nodes[ti].y - self.nodes[fi].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let displacement = dist - spring_rest;
                let force = spring_k * displacement;
                let fdx = (dx / dist) * force;
                let fdy = (dy / dist) * force;
                fx[fi] += fdx;
                fy[fi] += fdy;
                fx[ti] -= fdx;
                fy[ti] -= fdy;
            }
        }

        // Center gravity (gentle pull towards origin)
        for i in 0..n {
            fx[i] -= self.nodes[i].x * 0.001;
            fy[i] -= self.nodes[i].y * 0.001;
        }

        // Apply forces (skip dragged node)
        for i in 0..n {
            if Some(self.nodes[i].id) == self.dragged_node {
                self.nodes[i].vx = 0.0;
                self.nodes[i].vy = 0.0;
                continue;
            }
            self.nodes[i].vx = (self.nodes[i].vx + fx[i]) * damping;
            self.nodes[i].vy = (self.nodes[i].vy + fy[i]) * damping;

            // Clamp speed
            let speed = (self.nodes[i].vx.powi(2) + self.nodes[i].vy.powi(2)).sqrt();
            if speed > max_speed {
                self.nodes[i].vx = (self.nodes[i].vx / speed) * max_speed;
                self.nodes[i].vy = (self.nodes[i].vy / speed) * max_speed;
            }

            self.nodes[i].x += self.nodes[i].vx;
            self.nodes[i].y += self.nodes[i].vy;
        }
    }

    /// Hit test: find node at world coordinates.
    fn node_at(&self, wx: f64, wy: f64) -> Option<usize> {
        for node in self.nodes.iter().rev() {
            let dx = wx - node.x;
            let dy = wy - node.y;
            if dx * dx + dy * dy <= node.radius * node.radius {
                return Some(node.id);
            }
        }
        None
    }

    /// Convert screen coordinates to world coordinates.
    fn screen_to_world(&self, sx: f64, sy: f64, width: f64, height: f64) -> (f64, f64) {
        let wx = (sx - width / 2.0) / self.zoom + self.cam_x;
        let wy = (sy - height / 2.0) / self.zoom + self.cam_y;
        (wx, wy)
    }
}

// ═══════════════════════════════════════════════
//  Build the Graph View Widget
// ═══════════════════════════════════════════════

/// Creates the full interactive graph view widget for the given directory.
pub fn build_graph_view(
    current_path: Rc<RefCell<PathBuf>>,
    _config: Rc<RefCell<AppConfig>>,
) -> DrawingArea {
    let state = Rc::new(RefCell::new(GraphState::new()));

    // Initialise with root node (expanded)
    {
        let path = current_path.borrow().clone();
        let mut s = state.borrow_mut();
        let root_id = s.add_root(&path);
        s.expand_node(root_id);
    }

    let area = DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .css_classes(vec!["graph-view".to_string()])
        .build();

    // ── Draw callback ──
    {
        let state_c = state.clone();
        area.set_draw_func(move |_area, cr, width, height| {
            let s = state_c.borrow();
            draw_graph(cr, &s, width as f64, height as f64);
        });
    }

    // ── Physics animation tick ──
    {
        let state_c = state.clone();
        let area_c = area.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            state_c.borrow_mut().physics_step();
            area_c.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    // ── Scroll → zoom ──
    {
        let scroll_ctrl = EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::VERTICAL | gtk4::EventControllerScrollFlags::DISCRETE,
        );
        let state_c = state.clone();
        scroll_ctrl.connect_scroll(move |_, _dx, dy| {
            let mut s = state_c.borrow_mut();
            let factor = if dy < 0.0 { 1.1 } else { 1.0 / 1.1 };
            s.zoom = (s.zoom * factor).clamp(0.1, 5.0);
            glib::Propagation::Stop
        });
        area.add_controller(scroll_ctrl);
    }

    // ── Mouse motion → hover detection ──
    {
        let motion_ctrl = EventControllerMotion::new();
        let state_c = state.clone();
        let area_c = area.clone();
        motion_ctrl.connect_motion(move |_, x, y| {
            let mut s = state_c.borrow_mut();
            let w = area_c.width() as f64;
            let h = area_c.height() as f64;
            let (wx, wy) = s.screen_to_world(x, y, w, h);
            s.hovered_node = s.node_at(wx, wy);
        });
        area.add_controller(motion_ctrl);
    }

    // ── Click → expand/collapse directories ──
    {
        let click_ctrl = GestureClick::builder().button(1).build();
        let state_c = state.clone();
        let area_c = area.clone();
        click_ctrl.connect_released(move |_, _n, x, y| {
            let mut s = state_c.borrow_mut();
            let w = area_c.width() as f64;
            let h = area_c.height() as f64;
            let (wx, wy) = s.screen_to_world(x, y, w, h);
            if let Some(nid) = s.node_at(wx, wy) {
                let is_dir = s.nodes.iter().find(|n| n.id == nid).map(|n| n.is_dir);
                let is_expanded = s.nodes.iter().find(|n| n.id == nid).map(|n| n.is_expanded);
                if is_dir == Some(true) {
                    if is_expanded == Some(true) {
                        s.collapse_node(nid);
                    } else {
                        s.expand_node(nid);
                    }
                } else {
                    // Open file on click
                    if let Some(node) = s.nodes.iter().find(|n| n.id == nid) {
                        let _ = open::that(&node.path);
                    }
                }
            }
        });
        area.add_controller(click_ctrl);
    }

    // ── Drag → move nodes or pan camera ──
    {
        let drag_ctrl = GestureDrag::builder().button(1).build();
        let state_c = state.clone();
        let area_c = area.clone();

        let state_c2 = state_c.clone();
        let area_c2 = area_c.clone();
        drag_ctrl.connect_drag_begin(move |_, x, y| {
            let mut s = state_c2.borrow_mut();
            let w = area_c2.width() as f64;
            let h = area_c2.height() as f64;
            let (wx, wy) = s.screen_to_world(x, y, w, h);
            if let Some(nid) = s.node_at(wx, wy) {
                if let Some(node) = s.nodes.iter().find(|n| n.id == nid) {
                    let (nx, ny) = (node.x, node.y);
                    s.dragged_node = Some(nid);
                    s.drag_offset_x = wx - nx;
                    s.drag_offset_y = wy - ny;
                }
            } else {
                // Pan mode
                s.is_panning = true;
                s.pan_start_x = x;
                s.pan_start_y = y;
                s.cam_start_x = s.cam_x;
                s.cam_start_y = s.cam_y;
            }
        });

        let state_c3 = state_c.clone();
        let area_c3 = area_c.clone();
        drag_ctrl.connect_drag_update(move |gesture, dx, dy| {
            let mut s = state_c3.borrow_mut();
            if let Some(nid) = s.dragged_node {
                if let Some((start_x, start_y)) = gesture.start_point() {
                    let w = area_c3.width() as f64;
                    let h = area_c3.height() as f64;
                    let (wx, wy) = s.screen_to_world(start_x + dx, start_y + dy, w, h);
                    let off_x = s.drag_offset_x;
                    let off_y = s.drag_offset_y;
                    if let Some(node) = s.nodes.iter_mut().find(|n| n.id == nid) {
                        node.x = wx - off_x;
                        node.y = wy - off_y;
                        node.vx = 0.0;
                        node.vy = 0.0;
                    }
                }
            } else if s.is_panning {
                s.cam_x = s.cam_start_x - dx / s.zoom;
                s.cam_y = s.cam_start_y - dy / s.zoom;
            }
        });

        let state_c4 = state_c.clone();
        drag_ctrl.connect_drag_end(move |_, _, _| {
            let mut s = state_c4.borrow_mut();
            s.dragged_node = None;
            s.is_panning = false;
        });

        area.add_controller(drag_ctrl);
    }

    area
}

// ═══════════════════════════════════════════════
//  Cairo Drawing
// ═══════════════════════════════════════════════

fn draw_graph(cr: &gtk4::cairo::Context, state: &GraphState, width: f64, height: f64) {
    // Background (transparent — CSS handles it)
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint().ok();

    // Transform: centre + zoom + camera offset
    cr.translate(width / 2.0, height / 2.0);
    cr.scale(state.zoom, state.zoom);
    cr.translate(-state.cam_x, -state.cam_y);

    // Draw edges
    cr.set_line_width(1.5 / state.zoom.max(0.5));
    for edge in &state.edges {
        let from = state.nodes.iter().find(|n| n.id == edge.from);
        let to = state.nodes.iter().find(|n| n.id == edge.to);
        if let (Some(f), Some(t)) = (from, to) {
            cr.set_source_rgba(0.5, 0.5, 0.6, 0.3);
            cr.move_to(f.x, f.y);
            cr.line_to(t.x, t.y);
            cr.stroke().ok();
        }
    }

    // Draw nodes
    for node in &state.nodes {
        let is_hovered = state.hovered_node == Some(node.id);
        let r = if is_hovered {
            node.radius * 1.2
        } else {
            node.radius
        };

        // Node circle
        cr.arc(node.x, node.y, r, 0.0, 2.0 * PI);

        // Fill
        let alpha = if is_hovered { 1.0 } else { 0.85 };
        cr.set_source_rgba(node.color.r, node.color.g, node.color.b, alpha);
        cr.fill_preserve().ok();

        // Stroke
        let stroke_alpha = if is_hovered { 0.9 } else { 0.4 };
        cr.set_source_rgba(1.0, 1.0, 1.0, stroke_alpha);
        cr.set_line_width(if is_hovered { 2.5 } else { 1.2 });
        cr.stroke().ok();

        // Expand indicator for directories
        if node.is_dir && !node.is_expanded {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.7);
            cr.arc(node.x, node.y, 4.0, 0.0, 2.0 * PI);
            cr.fill().ok();
        }

        // Label
        let font_size = if is_hovered { 11.0 } else { 9.0 };
        cr.set_font_size(font_size / state.zoom.max(0.3));
        cr.set_source_rgba(0.9, 0.9, 0.95, if is_hovered { 1.0 } else { 0.8 });

        let label = truncate_label(&node.label, 18);
        if let Ok(extents) = cr.text_extents(&label) {
            cr.move_to(node.x - extents.width() / 2.0, node.y + r + 14.0);
            cr.show_text(&label).ok();
        }
    }
}

// ═══════════════════════════════════════════════
//  Color Helpers
// ═══════════════════════════════════════════════

fn dir_color() -> NodeColor {
    NodeColor {
        r: 0.54,
        g: 0.71,
        b: 0.98,
    } // #89B4FA – Catppuccin blue
}

fn file_color_for_ext(ext: &str) -> NodeColor {
    match ext {
        "rs" => NodeColor {
            r: 0.87,
            g: 0.52,
            b: 0.26,
        }, // Rust orange
        "py" => NodeColor {
            r: 0.36,
            g: 0.65,
            b: 0.85,
        }, // Python blue
        "js" | "ts" => NodeColor {
            r: 0.95,
            g: 0.85,
            b: 0.30,
        }, // JS yellow
        "c" | "cpp" | "h" => NodeColor {
            r: 0.40,
            g: 0.60,
            b: 0.80,
        },
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => NodeColor {
            r: 0.65,
            g: 0.85,
            b: 0.55,
        }, // green
        "mp3" | "flac" | "ogg" | "wav" => NodeColor {
            r: 0.80,
            g: 0.55,
            b: 0.80,
        }, // purple
        "mp4" | "mkv" | "avi" | "mov" | "webm" => NodeColor {
            r: 0.90,
            g: 0.45,
            b: 0.45,
        }, // red
        "md" | "txt" | "log" => NodeColor {
            r: 0.70,
            g: 0.70,
            b: 0.75,
        }, // grey
        "json" | "toml" | "yaml" | "yml" | "xml" => NodeColor {
            r: 0.55,
            g: 0.78,
            b: 0.65,
        }, // teal
        _ => NodeColor {
            r: 0.60,
            g: 0.60,
            b: 0.65,
        }, // default grey
    }
}

fn truncate_label(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

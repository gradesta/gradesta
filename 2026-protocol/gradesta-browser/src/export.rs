//! HTML export functionality for gradesta graphs
//!
//! Exports a section of the graph as a standalone HTML file with all content
//! embedded (images as base64, text inline, audio as base64).

use std::collections::{HashMap, HashSet};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::graph::{GraphState, Vertex};
use crate::state::Direction;

/// State for the export panel UI
#[derive(Clone, Debug, Default)]
pub struct ExportState {
    /// Which directions to include in the export
    pub directions: HashSet<Direction>,
}

impl ExportState {
    pub fn new() -> Self {
        Self {
            directions: HashSet::new(),
        }
    }

    pub fn toggle_direction(&mut self, dir: Direction) {
        if self.directions.contains(&dir) {
            self.directions.remove(&dir);
        } else {
            self.directions.insert(dir);
        }
    }

    pub fn is_direction_enabled(&self, dir: Direction) -> bool {
        self.directions.contains(&dir)
    }
}

/// Collected vertices for export, organized by position
pub struct ExportData {
    /// All vertices to export, keyed by their relative position
    pub vertices: HashMap<(i32, i32, i32), ExportVertex>,
    /// Bounding box
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_z: i32,
    pub max_z: i32,
}

/// A vertex prepared for export
pub struct ExportVertex {
    pub id: u64,
    pub position: (i32, i32, i32),
    pub content: ExportContent,
    /// Edges to adjacent vertices (by position, not id)
    pub edges: [Option<(i32, i32, i32)>; 6],
}

/// Content type for export
pub enum ExportContent {
    Text(String),
    Image { mime: String, data: Vec<u8> },
    Audio { mime: String, data: Vec<u8>, transcript: Option<String> },
    Portal(String),
    Other { mime: String, data: Vec<u8> },
}

/// Collect all vertices reachable from starting vertex following the given directions
pub fn collect_vertices_for_export(
    graph: &GraphState,
    start_vertex: u64,
    directions: &HashSet<Direction>,
) -> ExportData {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut positions: HashMap<u64, (i32, i32, i32)> = HashMap::new();
    let mut vertices: HashMap<(i32, i32, i32), ExportVertex> = HashMap::new();

    // BFS to collect all reachable vertices
    let mut queue: Vec<(u64, (i32, i32, i32))> = vec![(start_vertex, (0, 0, 0))];
    visited.insert(start_vertex);
    positions.insert(start_vertex, (0, 0, 0));

    while let Some((vertex_id, pos)) = queue.pop() {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Check each direction
            for &dir in Direction::all() {
                if !directions.contains(&dir) {
                    continue;
                }

                let edge_idx = dir.to_edge_index();
                let neighbor_id = vertex.edges[edge_idx];
                if neighbor_id == 0 || visited.contains(&neighbor_id) {
                    continue;
                }

                let new_pos = offset_position(pos, dir);
                visited.insert(neighbor_id);
                positions.insert(neighbor_id, new_pos);
                queue.push((neighbor_id, new_pos));
            }
        }
    }

    // Convert to ExportVertex
    let mut min_x = 0i32;
    let mut max_x = 0i32;
    let mut min_y = 0i32;
    let mut max_y = 0i32;
    let mut min_z = 0i32;
    let mut max_z = 0i32;

    for (&vertex_id, &pos) in &positions {
        min_x = min_x.min(pos.0);
        max_x = max_x.max(pos.0);
        min_y = min_y.min(pos.1);
        max_y = max_y.max(pos.1);
        min_z = min_z.min(pos.2);
        max_z = max_z.max(pos.2);

        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            let content = extract_content(vertex);
            let mut edges = [None; 6];

            // Map edges to positions
            for (i, &neighbor_id) in vertex.edges.iter().enumerate() {
                if neighbor_id != 0 {
                    if let Some(&neighbor_pos) = positions.get(&neighbor_id) {
                        edges[i] = Some(neighbor_pos);
                    }
                }
            }

            vertices.insert(pos, ExportVertex {
                id: vertex_id,
                position: pos,
                content,
                edges,
            });
        }
    }

    ExportData {
        vertices,
        min_x,
        max_x,
        min_y,
        max_y,
        min_z,
        max_z,
    }
}

fn offset_position(pos: (i32, i32, i32), dir: Direction) -> (i32, i32, i32) {
    match dir {
        Direction::West => (pos.0 - 1, pos.1, pos.2),
        Direction::East => (pos.0 + 1, pos.1, pos.2),
        Direction::North => (pos.0, pos.1 - 1, pos.2),
        Direction::South => (pos.0, pos.1 + 1, pos.2),
        Direction::Up => (pos.0, pos.1, pos.2 + 1),
        Direction::Down => (pos.0, pos.1, pos.2 - 1),
    }
}

fn extract_content(vertex: &Vertex) -> ExportContent {
    let mime = vertex.mime.as_deref().unwrap_or("text/plain");

    // Check for portal
    if mime == "text/gradesta-url" {
        return ExportContent::Portal(String::from_utf8_lossy(&vertex.label).to_string());
    }

    // Check for audio with transcript
    if mime.starts_with("audio/") {
        let transcript = vertex.layers.get(&1)
            .filter(|l| l.mime.starts_with("text/"))
            .map(|l| String::from_utf8_lossy(&l.data).to_string());
        return ExportContent::Audio {
            mime: mime.to_string(),
            data: vertex.label.clone(),
            transcript,
        };
    }

    // Check for image (prefer layer 2 full-res if available)
    if mime.starts_with("image/") {
        if let Some(layer2) = vertex.layers.get(&2) {
            if layer2.mime.starts_with("image/") {
                return ExportContent::Image {
                    mime: layer2.mime.clone(),
                    data: layer2.data.clone(),
                };
            }
        }
        return ExportContent::Image {
            mime: mime.to_string(),
            data: vertex.label.clone(),
        };
    }

    // Check for text
    if mime.starts_with("text/") && mime != "text/x-url" {
        return ExportContent::Text(String::from_utf8_lossy(&vertex.label).to_string());
    }

    // Other content
    ExportContent::Other {
        mime: mime.to_string(),
        data: vertex.label.clone(),
    }
}

/// Generate standalone HTML from export data
pub fn generate_html(data: &ExportData, title: &str) -> String {
    let mut html = String::new();

    // HTML header
    html.push_str(&format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        :root {{
            --bg-color: #1a1a2e;
            --card-bg: #16213e;
            --text-color: #eee;
            --border-color: #0f3460;
            --link-color: #e94560;
        }}
        * {{
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 20px;
            line-height: 1.6;
        }}
        .grid-container {{
            display: grid;
            gap: 10px;
            justify-content: center;
        }}
        .cell {{
            background: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 15px;
            min-width: 200px;
            max-width: 400px;
            overflow: hidden;
        }}
        .cell.current {{
            border-color: var(--link-color);
            border-width: 2px;
        }}
        .cell-content {{
            max-height: 300px;
            overflow: auto;
        }}
        .cell img {{
            max-width: 100%;
            height: auto;
            border-radius: 4px;
        }}
        .cell audio {{
            width: 100%;
        }}
        .transcript {{
            font-style: italic;
            color: #aaa;
            margin-top: 10px;
            padding-top: 10px;
            border-top: 1px solid var(--border-color);
        }}
        .portal {{
            color: var(--link-color);
            text-decoration: none;
        }}
        .portal:hover {{
            text-decoration: underline;
        }}
        .stack-indicator {{
            font-size: 0.8em;
            color: #888;
            margin-bottom: 5px;
        }}
        .layer-nav {{
            display: flex;
            gap: 5px;
            margin-bottom: 10px;
        }}
        .layer-nav button {{
            background: var(--border-color);
            border: none;
            color: var(--text-color);
            padding: 5px 10px;
            border-radius: 4px;
            cursor: pointer;
        }}
        .layer-nav button:hover {{
            background: var(--link-color);
        }}
        pre {{
            white-space: pre-wrap;
            word-wrap: break-word;
            margin: 0;
        }}
        h1 {{
            text-align: center;
            margin-bottom: 30px;
        }}
    </style>
</head>
<body>
    <h1>{}</h1>
"#, html_escape(title), html_escape(title)));

    // Generate grid - we'll organize by z-level (stacks)
    let z_levels: Vec<i32> = (data.min_z..=data.max_z).collect();

    for z in z_levels {
        if data.max_z > data.min_z {
            html.push_str(&format!(r#"    <h2>Level {} ({})</h2>
"#, z, if z > 0 { "Up" } else if z < 0 { "Down" } else { "Base" }));
        }

        // Calculate grid dimensions for this z-level
        let cols = (data.max_x - data.min_x + 1) as usize;

        html.push_str(&format!(r#"    <div class="grid-container" style="grid-template-columns: repeat({}, minmax(200px, 400px));">
"#, cols));

        for y in data.min_y..=data.max_y {
            for x in data.min_x..=data.max_x {
                let pos = (x, y, z);
                if let Some(vertex) = data.vertices.get(&pos) {
                    let is_origin = x == 0 && y == 0 && z == 0;
                    let class = if is_origin { "cell current" } else { "cell" };

                    html.push_str(&format!(r#"        <div class="{}">
"#, class));

                    // Show stack navigation if there are up/down connections
                    let up_idx = Direction::Up.to_edge_index();
                    let down_idx = Direction::Down.to_edge_index();
                    if vertex.edges[up_idx].is_some() || vertex.edges[down_idx].is_some() {
                        html.push_str(r#"            <div class="stack-indicator">"#);
                        if vertex.edges[up_idx].is_some() {
                            html.push_str("⬆ ");
                        }
                        if vertex.edges[down_idx].is_some() {
                            html.push_str("⬇");
                        }
                        html.push_str("</div>\n");
                    }

                    html.push_str(r#"            <div class="cell-content">
"#);

                    match &vertex.content {
                        ExportContent::Text(text) => {
                            html.push_str(&format!("                <pre>{}</pre>\n", html_escape(text)));
                        }
                        ExportContent::Image { mime, data } => {
                            let b64 = BASE64.encode(data);
                            html.push_str(&format!(
                                "                <img src=\"data:{};base64,{}\" alt=\"Image\">\n",
                                mime, b64
                            ));
                        }
                        ExportContent::Audio { mime, data, transcript } => {
                            let b64 = BASE64.encode(data);
                            html.push_str(&format!(
                                "                <audio controls src=\"data:{};base64,{}\"></audio>\n",
                                mime, b64
                            ));
                            if let Some(t) = transcript {
                                html.push_str(&format!(
                                    "                <div class=\"transcript\">{}</div>\n",
                                    html_escape(t)
                                ));
                            }
                        }
                        ExportContent::Portal(url) => {
                            html.push_str(&format!(
                                "                <a class=\"portal\" href=\"{}\">Portal: {}</a>\n",
                                html_escape(url), html_escape(url)
                            ));
                        }
                        ExportContent::Other { mime, data } => {
                            html.push_str(&format!(
                                "                <p>Content type: {} ({} bytes)</p>\n",
                                html_escape(mime), data.len()
                            ));
                        }
                    }

                    html.push_str("            </div>\n");
                    html.push_str("        </div>\n");
                } else {
                    // Empty cell placeholder
                    html.push_str("        <div></div>\n");
                }
            }
        }

        html.push_str("    </div>\n");
    }

    // HTML footer
    html.push_str(r#"</body>
</html>
"#);

    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Save HTML to file using native file dialog
pub fn save_html_file(html: &str) -> Result<String, String> {
    // Use rfd (Rust File Dialog) if available, otherwise fall back to a default path
    let path = if let Some(path) = rfd::FileDialog::new()
        .add_filter("HTML", &["html", "htm"])
        .set_file_name("export.html")
        .save_file()
    {
        path
    } else {
        return Err("File dialog cancelled".to_string());
    };

    std::fs::write(&path, html)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

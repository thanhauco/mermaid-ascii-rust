use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::parser::{GraphDirection, GraphProperties, StyleClass};
use crate::render::drawing::Drawing;
use crate::render::geom::{DrawingCoord, GridCoord};

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub border_padding: i32,
    pub use_ascii: bool,
    pub show_coords: bool,
}

pub fn render_properties(properties: &mut GraphProperties, options: &RenderOptions) -> Result<String> {
    let mut graph = Graph::from_properties(properties, options.clone())?;
    graph.set_style_classes();
    graph.create_mapping()?;
    let drawing = graph.draw_nodes();
    Ok(drawing.to_string())
}

#[derive(Clone, Debug)]
struct Node {
    name: String,
    drawing: Option<Drawing>,
    drawing_coord: Option<DrawingCoord>,
    grid_coord: Option<GridCoord>,
    index: usize,
    style_class_name: Option<String>,
    style_class: Option<StyleClass>,
}

impl Node {
    fn new(name: String, index: usize) -> Node {
        Node {
            name,
            drawing: None,
            drawing_coord: None,
            grid_coord: None,
            index,
            style_class_name: None,
            style_class: None,
        }
    }
}

#[derive(Clone, Debug)]
struct Edge {
    from: usize,
    to: usize,
    text: String,
}

#[derive(Clone, Debug)]
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    drawing: Drawing,
    grid: HashMap<GridCoord, usize>,
    column_width: HashMap<i32, i32>,
    row_height: HashMap<i32, i32>,
    padding_x: i32,
    padding_y: i32,
    style_classes: HashMap<String, StyleClass>,
    style_type: String,
    direction: GraphDirection,
    options: RenderOptions,
    offset_x: i32,
    offset_y: i32,
}

impl Graph {
    fn from_properties(properties: &GraphProperties, options: RenderOptions) -> Result<Graph> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut node_lookup: HashMap<String, usize> = HashMap::new();
        let mut edges = Vec::new();

        for (node_name, children) in properties.data.iter() {
            let parent_index = *node_lookup.entry(node_name.clone()).or_insert_with(|| {
                let idx = nodes.len();
                nodes.push(Node::new(node_name.clone(), idx));
                idx
            });

            for text_edge in children {
                let child_index = *node_lookup
                    .entry(text_edge.child.name.clone())
                    .or_insert_with(|| {
                        let idx = nodes.len();
                        let mut child_node = Node::new(text_edge.child.name.clone(), idx);
                        if !text_edge.child.style_class.clone().unwrap_or_default().is_empty() {
                            child_node.style_class_name = text_edge.child.style_class.clone();
                        } else if let Some(class) = text_edge.child.style_class.clone() {
                            if !class.is_empty() {
                                child_node.style_class_name = Some(class);
                            }
                        }
                        nodes.push(child_node);
                        idx
                    });

                if let Some(class) = text_edge.parent.style_class.clone() {
                    if !class.is_empty() {
                        nodes[parent_index].style_class_name = Some(class);
                    }
                }
                if let Some(class) = text_edge.child.style_class.clone() {
                    if !class.is_empty() {
                        nodes[child_index].style_class_name = Some(class);
                    }
                }

                edges.push(Edge {
                    from: parent_index,
                    to: child_index,
                    text: text_edge.label.clone(),
                });
            }
        }

        Ok(Graph {
            nodes,
            edges,
            drawing: Drawing::empty(),
            grid: HashMap::new(),
            column_width: HashMap::new(),
            row_height: HashMap::new(),
            padding_x: properties.padding_x,
            padding_y: properties.padding_y,
            style_classes: HashMap::new(),
            style_type: properties.style_type.clone(),
            direction: properties.graph_direction,
            options,
            offset_x: 0,
            offset_y: 0,
        })
    }

    fn set_style_classes(&mut self) {
        self.style_classes = self.style_classes.clone();
    }

    fn create_mapping(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(anyhow!("no nodes to render"));
        }

        let mut highest_position_per_level = vec![0; 128];
        let mut nodes_found: HashSet<String> = HashSet::new();
        let mut root_nodes: Vec<usize> = Vec::new();

        for node in &self.nodes {
            if nodes_found.insert(node.name.clone()) {
                root_nodes.push(node.index);
            }
            for child in self.get_children(node.index) {
                nodes_found.insert(self.nodes[child].name.clone());
            }
        }

        for idx in root_nodes {
            let requested = if self.direction == GraphDirection::Lr {
                GridCoord {
                    x: 0,
                    y: highest_position_per_level[0],
                }
            } else {
                GridCoord {
                    x: highest_position_per_level[0],
                    y: 0,
                }
            };
            let reserved = self.reserve_spot_in_grid(idx, requested);
            self.nodes[idx].grid_coord = Some(reserved);
            highest_position_per_level[0] += 4;
        }

        for idx in 0..self.nodes.len() {
            let Some(coord) = self.nodes[idx].grid_coord else {
                continue;
            };
            let child_level = if self.direction == GraphDirection::Lr {
                coord.x + 4
            } else {
                coord.y + 4
            };
            let mut highest_position = highest_position_per_level[child_level as usize];
            for child_idx in self.get_children(idx) {
                if self.nodes[child_idx].grid_coord.is_some() {
                    continue;
                }
                let requested = if self.direction == GraphDirection::Lr {
                    GridCoord {
                        x: child_level,
                        y: highest_position,
                    }
                } else {
                    GridCoord {
                        x: highest_position,
                        y: child_level,
                    }
                };
                let reserved = self.reserve_spot_in_grid(child_idx, requested);
                self.nodes[child_idx].grid_coord = Some(reserved);
                highest_position += 4;
                highest_position_per_level[child_level as usize] = highest_position;
            }
        }

        for idx in 0..self.nodes.len() {
            if let Some(coord) = self.nodes[idx].grid_coord {
                self.set_column_width(idx, coord);
            }
        }

        self.set_drawing_size_to_grid_constraints();

        for idx in 0..self.nodes.len() {
            let coord = self.nodes[idx]
                .grid_coord
                .expect("node should have grid coord");
            let drawing_coord = self.grid_to_drawing_coord(coord, None);
            self.nodes[idx].drawing_coord = Some(drawing_coord);
            self.nodes[idx].drawing = Some(draw_box(&self.nodes[idx], self));
        }

        Ok(())
    }

    fn get_children(&self, node_index: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter_map(|edge| {
                if edge.from == node_index {
                    Some(edge.to)
                } else {
                    None
                }
            })
            .collect()
    }

    fn reserve_spot_in_grid(&mut self, node_index: usize, requested: GridCoord) -> GridCoord {
        if self.grid.contains_key(&requested) {
            let next = if self.direction == GraphDirection::Lr {
                GridCoord {
                    x: requested.x,
                    y: requested.y + 4,
                }
            } else {
                GridCoord {
                    x: requested.x + 4,
                    y: requested.y,
                }
            };
            return self.reserve_spot_in_grid(node_index, next);
        }
        for dx in 0..3 {
            for dy in 0..3 {
                let coord = GridCoord {
                    x: requested.x + dx,
                    y: requested.y + dy,
                };
                self.grid.insert(coord, node_index);
            }
        }
        requested
    }

    fn set_column_width(&mut self, node_index: usize, coord: GridCoord) {
        let text_len = self.nodes[node_index].name.chars().count() as i32;
        let col1 = 1;
        let col2 = 2 * self.options.border_padding + text_len;
        let col3 = 1;
        let cols = [col1, col2, col3];
        let rows = [
            1,
            1 + 2 * self.options.border_padding,
            1,
        ];

        for (idx, col) in cols.iter().enumerate() {
            let x_coord = coord.x + idx as i32;
            let entry = self.column_width.entry(x_coord).or_insert(0);
            *entry = (*entry).max(*col);
        }

        for (idx, row) in rows.iter().enumerate() {
            let y_coord = coord.y + idx as i32;
            let entry = self.row_height.entry(y_coord).or_insert(0);
            *entry = (*entry).max(*row);
        }

        if coord.x > 0 {
            self.column_width
                .entry(coord.x - 1)
                .and_modify(|w| *w = (*w).max(self.padding_x))
                .or_insert(self.padding_x);
        }
        if coord.y > 0 {
            self.row_height
                .entry(coord.y - 1)
                .and_modify(|h| *h = (*h).max(self.padding_y))
                .or_insert(self.padding_y);
        }
    }

    fn set_drawing_size_to_grid_constraints(&mut self) {
        let max_x: i32 = self.column_width.values().sum();
        let max_y: i32 = self.row_height.values().sum();
        self.drawing.increase_size(
            max_x.saturating_sub(1) as usize,
            max_y.saturating_sub(1) as usize,
        );
    }

    fn grid_to_drawing_coord(&self, coord: GridCoord, direction: Option<crate::render::geom::Direction>) -> DrawingCoord {
        let target = if let Some(dir) = direction {
            coord.direction(dir)
        } else {
            coord
        };
        let mut x = 0;
        for col in 0..target.x {
            x += self.column_width.get(&col).copied().unwrap_or(0);
        }
        let mut y = 0;
        for row in 0..target.y {
            y += self.row_height.get(&row).copied().unwrap_or(0);
        }
        DrawingCoord {
            x: x + self.column_width.get(&target.x).copied().unwrap_or(0) / 2 + self.offset_x,
            y: y + self.row_height.get(&target.y).copied().unwrap_or(0) / 2 + self.offset_y,
        }
    }

    fn draw_nodes(&mut self) -> Drawing {
        let mut drawing = self.drawing.clone();
        for node in &self.nodes {
            if let (Some(coord), Some(box_drawing)) = (node.drawing_coord, node.drawing.clone()) {
                drawing = Drawing::merge_with(&drawing, coord, &[box_drawing], self.options.use_ascii);
            }
        }
        drawing
    }
}

fn draw_box(node: &Node, graph: &Graph) -> Drawing {
    let coord = node.grid_coord.expect("node must have coord");
    let mut width = 0;
    for i in 0..2 {
        width += graph.column_width.get(&(coord.x + i)).copied().unwrap_or(0);
    }
    let mut height = 0;
    for i in 0..2 {
        height += graph.row_height.get(&(coord.y + i)).copied().unwrap_or(0);
    }

    let mut drawing = Drawing::new(width as usize, height as usize);

    if graph.options.use_ascii {
        for x in 1..width {
            drawing.set(DrawingCoord { x, y: 0 }, "-");
            drawing.set(DrawingCoord { x, y: height }, "-");
        }
        for y in 1..height {
            drawing.set(DrawingCoord { x: 0, y }, "|");
            drawing.set(DrawingCoord { x: width, y }, "|");
        }
        drawing.set(DrawingCoord { x: 0, y: 0 }, "+");
        drawing.set(DrawingCoord { x: width, y: 0 }, "+");
        drawing.set(DrawingCoord { x: 0, y: height }, "+");
        drawing.set(DrawingCoord { x: width, y: height }, "+");
    } else {
        for x in 1..width {
            drawing.set(DrawingCoord { x, y: 0 }, "─");
            drawing.set(DrawingCoord { x, y: height }, "─");
        }
        for y in 1..height {
            drawing.set(DrawingCoord { x: 0, y }, "│");
            drawing.set(DrawingCoord { x: width, y }, "│");
        }
        drawing.set(DrawingCoord { x: 0, y: 0 }, "┌");
        drawing.set(DrawingCoord { x: width, y: 0 }, "┐");
        drawing.set(DrawingCoord { x: 0, y: height }, "└");
        drawing.set(DrawingCoord { x: width, y: height }, "┘");
    }

    let text_y = height / 2;
    let text_x = width / 2 - (node.name.chars().count() as i32) / 2 + 1;
    drawing.draw_text(
        DrawingCoord {
            x: text_x,
            y: text_y,
        },
        &node.name,
    );

    drawing
}

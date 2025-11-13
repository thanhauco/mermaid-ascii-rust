use std::cmp::max;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

use anyhow::{anyhow, Result};

use crate::parser::{GraphDirection, GraphProperties, StyleClass};
use crate::render::drawing::Drawing;
use crate::render::geom::{
    determine_direction, Direction, DrawingCoord, GenericCoord, GridCoord,
};

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub border_padding: i32,
    pub use_ascii: bool,
    pub show_coords: bool,
}

pub fn render_properties(
    properties: &GraphProperties,
    options: &RenderOptions,
) -> Result<String> {
    let mut graph = Graph::new(properties, options.clone());
    graph.layout()?;
    let mut drawing = graph.draw();
    if options.show_coords {
        drawing = graph.with_coords_overlay(drawing);
    }
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
    path: Vec<GridCoord>,
    label_line: Vec<GridCoord>,
    start_dir: Direction,
    end_dir: Direction,
}

impl Edge {
    fn new(from: usize, to: usize, text: String) -> Edge {
        Edge {
            from,
            to,
            text,
            path: Vec::new(),
            label_line: Vec::new(),
            start_dir: Direction::Right,
            end_dir: Direction::Left,
        }
    }
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
    fn new(properties: &GraphProperties, options: RenderOptions) -> Graph {
        let mut nodes: Vec<Node> = Vec::new();
        let mut node_lookup: HashMap<String, usize> = HashMap::new();
        let mut edges: Vec<Edge> = Vec::new();

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
                        let mut child =
                            Node::new(text_edge.child.name.clone(), idx);
                        if let Some(class) = &text_edge.child.style_class {
                            if !class.is_empty() {
                                child.style_class_name = Some(class.clone());
                            }
                        }
                        nodes.push(child);
                        idx
                    });

                if let Some(class) = &text_edge.parent.style_class {
                    if !class.is_empty() {
                        nodes[parent_index].style_class_name = Some(class.clone());
                    }
                }
                if let Some(class) = &text_edge.child.style_class {
                    if !class.is_empty() {
                        nodes[child_index].style_class_name = Some(class.clone());
                    }
                }

                edges.push(Edge::new(
                    parent_index,
                    child_index,
                    text_edge.label.clone(),
                ));
            }

            // Ensure standalone nodes (with no outgoing edges) still record style class
            if let Some(style_name) = &nodes[parent_index].style_class_name {
                if style_name.is_empty() {
                    nodes[parent_index].style_class_name = None;
                }
            }
        }

        Graph {
            nodes,
            edges,
            drawing: Drawing::empty(),
            grid: HashMap::new(),
            column_width: HashMap::new(),
            row_height: HashMap::new(),
            padding_x: properties.padding_x,
            padding_y: properties.padding_y,
            style_classes: properties.style_classes.clone(),
            style_type: properties.style_type.clone(),
            direction: properties.graph_direction,
            options,
            offset_x: 0,
            offset_y: 0,
        }
    }

    fn layout(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(anyhow!("no nodes to render"));
        }

        self.set_style_classes();
        self.create_mapping();

        for idx in 0..self.nodes.len() {
            if let Some(coord) = self.nodes[idx].grid_coord {
                self.set_column_width(idx, coord);
            }
        }

        for edge in &mut self.edges {
            self.determine_path(edge)?;
            self.increase_grid_size_for_path(&edge.path);
            self.determine_label_line(edge);
        }

        self.set_drawing_size_to_grid_constraints();

        for node in &mut self.nodes {
            if let Some(coord) = node.grid_coord {
                let drawing_coord = self.grid_to_drawing_coord(coord, None);
                node.drawing_coord = Some(drawing_coord);
                node.drawing = Some(draw_box(node, self));
            }
        }

        Ok(())
    }

    fn set_style_classes(&mut self) {
        for node in &mut self.nodes {
            if let Some(name) = &node.style_class_name {
                node.style_class = self.style_classes.get(name).cloned();
            }
        }
    }

    fn create_mapping(&mut self) {
        let mut highest_per_level: HashMap<i32, i32> = HashMap::new();
        let mut has_incoming = vec![false; self.nodes.len()];
        for edge in &self.edges {
            has_incoming[edge.to] = true;
        }

        let root_nodes: Vec<usize> = if has_incoming.iter().all(|x| *x) {
            (0..self.nodes.len()).collect()
        } else {
            has_incoming
                .iter()
                .enumerate()
                .filter_map(|(idx, has_parent)| {
                    if !has_parent {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect()
        };

        for idx in &root_nodes {
            let coord = if self.direction == GraphDirection::Lr {
                GridCoord {
                    x: 0,
                    y: *highest_per_level.entry(0).or_insert(0),
                }
            } else {
                GridCoord {
                    x: *highest_per_level.entry(0).or_insert(0),
                    y: 0,
                }
            };
            let reserved = self.reserve_spot_in_grid(*idx, coord);
            self.nodes[*idx].grid_coord = Some(reserved);
            let entry = highest_per_level.entry(0).or_insert(0);
            *entry += 4;
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
            let entry = highest_per_level.entry(child_level).or_insert(0);
            for child in self.get_children(idx) {
                if self.nodes[child].grid_coord.is_some() {
                    continue;
                }
                let requested = if self.direction == GraphDirection::Lr {
                    GridCoord {
                        x: child_level,
                        y: *entry,
                    }
                } else {
                    GridCoord {
                        x: *entry,
                        y: child_level,
                    }
                };
                let reserved = self.reserve_spot_in_grid(child, requested);
                self.nodes[child].grid_coord = Some(reserved);
                *entry += 4;
            }
        }
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

    fn reserve_spot_in_grid(
        &mut self,
        node_index: usize,
        requested: GridCoord,
    ) -> GridCoord {
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
        let cols = [
            1,
            2 * self.options.border_padding + text_len,
            1,
        ];
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

    fn determine_path(&mut self, edge: &mut Edge) -> Result<()> {
        let from_coord = self.nodes[edge.from]
            .grid_coord
            .ok_or_else(|| anyhow!("missing grid coord for node {}", edge.from))?;
        let to_coord = self.nodes[edge.to]
            .grid_coord
            .ok_or_else(|| anyhow!("missing grid coord for node {}", edge.to))?;

        let (preferred_dir, preferred_opposite, alt_dir, alt_opposite) =
            self.determine_start_and_end_dir(edge);

        let preferred_from = from_coord.direction(preferred_dir);
        let preferred_to = to_coord.direction(preferred_opposite);
        let alt_from = from_coord.direction(alt_dir);
        let alt_to = to_coord.direction(alt_opposite);

        let preferred_path = self.get_path(preferred_from, preferred_to)?;
        let preferred_path = merge_path(preferred_path);

        let alternative_path = self.get_path(alt_from, alt_to)?;
        let alternative_path = merge_path(alternative_path);

        if preferred_path.len() <= alternative_path.len() {
            edge.start_dir = preferred_dir;
            edge.end_dir = preferred_opposite;
            edge.path = preferred_path;
        } else {
            edge.start_dir = alt_dir;
            edge.end_dir = alt_opposite;
            edge.path = alternative_path;
        }

        Ok(())
    }

    fn determine_start_and_end_dir(
        &self,
        edge: &Edge,
    ) -> (Direction, Direction, Direction, Direction) {
        if edge.from == edge.to {
            return self.self_reference_direction();
        }

        let from = self.nodes[edge.from]
            .grid_coord
            .unwrap_or(GridCoord { x: 0, y: 0 });
        let to = self.nodes[edge.to]
            .grid_coord
            .unwrap_or(GridCoord { x: 0, y: 0 });

        let dir = determine_direction(
            GenericCoord {
                x: from.x,
                y: from.y,
            },
            GenericCoord { x: to.x, y: to.y },
        );

        let is_backwards = match self.direction {
            GraphDirection::Lr => matches!(
                dir,
                Direction::Left | Direction::UpperLeft | Direction::LowerLeft
            ),
            GraphDirection::Td => matches!(
                dir,
                Direction::Up | Direction::UpperLeft | Direction::UpperRight
            ),
        };

        match dir {
            Direction::LowerRight => {
                if self.direction == GraphDirection::Lr {
                    (Direction::Down, Direction::Left, Direction::Right, Direction::Up)
                } else {
                    (Direction::Right, Direction::Up, Direction::Down, Direction::Left)
                }
            }
            Direction::UpperRight => {
                if self.direction == GraphDirection::Lr {
                    (Direction::Up, Direction::Left, Direction::Right, Direction::Down)
                } else {
                    (Direction::Right, Direction::Down, Direction::Up, Direction::Left)
                }
            }
            Direction::LowerLeft => {
                if self.direction == GraphDirection::Lr {
                    (
                        Direction::Down,
                        Direction::Down,
                        Direction::Left,
                        Direction::Up,
                    )
                } else {
                    (Direction::Left, Direction::Up, Direction::Down, Direction::Right)
                }
            }
            Direction::UpperLeft => {
                if self.direction == GraphDirection::Lr {
                    (
                        Direction::Down,
                        Direction::Down,
                        Direction::Left,
                        Direction::Down,
                    )
                } else {
                    (
                        Direction::Right,
                        Direction::Right,
                        Direction::Up,
                        Direction::Right,
                    )
                }
            }
            _ => {
                if is_backwards {
                    match (self.direction, dir) {
                        (GraphDirection::Lr, Direction::Left) => (
                            Direction::Down,
                            Direction::Down,
                            Direction::Left,
                            Direction::Right,
                        ),
                        (GraphDirection::Td, Direction::Up) => (
                            Direction::Right,
                            Direction::Right,
                            Direction::Up,
                            Direction::Down,
                        ),
                        _ => (dir, dir.opposite(), dir, dir.opposite()),
                    }
                } else {
                    (dir, dir.opposite(), dir, dir.opposite())
                }
            }
        }
    }

    fn self_reference_direction(&self) -> (Direction, Direction, Direction, Direction) {
        match self.direction {
            GraphDirection::Lr => (
                Direction::Right,
                Direction::Down,
                Direction::Down,
                Direction::Right,
            ),
            GraphDirection::Td => (
                Direction::Down,
                Direction::Right,
                Direction::Right,
                Direction::Down,
            ),
        }
    }

    fn increase_grid_size_for_path(&mut self, path: &[GridCoord]) {
        for coord in path {
            self.column_width
                .entry(coord.x)
                .or_insert(self.padding_x / 2);
            self.row_height
                .entry(coord.y)
                .or_insert(self.padding_y / 2);
        }
    }

    fn determine_label_line(&mut self, edge: &mut Edge) {
        if edge.text.is_empty() || edge.path.len() < 2 {
            return;
        }

        let mut prev_step = edge.path[0];
        let mut largest_line = vec![edge.path[0], edge.path[1]];
        let mut largest_size = 0;
        for step in edge.path.iter().skip(1) {
            let line = vec![prev_step, *step];
            let width = self.calculate_line_width(&line);
            if width >= edge.text.len() as i32 {
                largest_line = line;
                break;
            } else if width > largest_size {
                largest_size = width;
                largest_line = line;
            }
            prev_step = *step;
        }

        let middle_x = if largest_line[0].x > largest_line[1].x {
            largest_line[1].x + (largest_line[0].x - largest_line[1].x) / 2
        } else {
            largest_line[0].x + (largest_line[1].x - largest_line[0].x) / 2
        };
        let column_entry = self.column_width.entry(middle_x).or_insert(0);
        *column_entry = max(*column_entry, edge.text.len() as i32 + 2);

        edge.label_line = largest_line;
    }

    fn calculate_line_width(&self, line: &[GridCoord]) -> i32 {
        line.iter()
            .map(|coord| self.column_width.get(&coord.x).copied().unwrap_or(0))
            .sum()
    }

    fn set_drawing_size_to_grid_constraints(&mut self) {
        let total_x: i32 = self.column_width.values().sum();
        let total_y: i32 = self.row_height.values().sum();
        self.drawing
            .increase_size(total_x.saturating_sub(1) as usize, total_y.saturating_sub(1) as usize);
    }

    fn grid_to_drawing_coord(
        &self,
        coord: GridCoord,
        direction: Option<Direction>,
    ) -> DrawingCoord {
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
            x: x + self
                .column_width
                .get(&target.x)
                .copied()
                .unwrap_or(self.padding_x)
                / 2
                + self.offset_x,
            y: y + self
                .row_height
                .get(&target.y)
                .copied()
                .unwrap_or(self.padding_y)
                / 2
                + self.offset_y,
        }
    }

    fn line_to_drawing(&self, line: &[GridCoord]) -> Vec<DrawingCoord> {
        line.iter()
            .map(|coord| self.grid_to_drawing_coord(*coord, None))
            .collect()
    }

    fn draw(&mut self) -> Drawing {
        let mut base = self.drawing.clone();
        for node in &self.nodes {
            if let (Some(coord), Some(node_drawing)) = (&node.drawing_coord, &node.drawing) {
                base.overlay(node_drawing, *coord, self.options.use_ascii);
            }
        }

        self.draw_edges(&mut base);
        base
    }

    fn draw_edges(&self, drawing: &mut Drawing) {
        let mut line_layer = self.drawing.blank_like();
        let mut corner_layer = self.drawing.blank_like();
        let mut arrow_head_layer = self.drawing.blank_like();
        let mut box_start_layer = self.drawing.blank_like();
        let mut label_layer = self.drawing.blank_like();

        for edge in &self.edges {
            if edge.path.is_empty() {
                continue;
            }
            let (path_lines, lines_drawn, line_dirs) = self.draw_path(&edge.path);
            line_layer.overlay(&path_lines, DrawingCoord { x: 0, y: 0 }, self.options.use_ascii);

            if let Some(first_line) = lines_drawn.first() {
                let box_start = self.draw_box_start(&edge.path, first_line);
                box_start_layer.overlay(
                    &box_start,
                    DrawingCoord { x: 0, y: 0 },
                    self.options.use_ascii,
                );
            }

            if let Some(last_line) = lines_drawn.last() {
                let fallback = *line_dirs.last().unwrap_or(&Direction::Right);
                let arrow_head = self.draw_arrow_head(last_line, fallback);
                arrow_head_layer.overlay(
                    &arrow_head,
                    DrawingCoord { x: 0, y: 0 },
                    self.options.use_ascii,
                );
            }

            let corners = self.draw_corners(&edge.path);
            corner_layer.overlay(
                &corners,
                DrawingCoord { x: 0, y: 0 },
                self.options.use_ascii,
            );

            let label = self.draw_arrow_label(edge);
            label_layer.overlay(
                &label,
                DrawingCoord { x: 0, y: 0 },
                self.options.use_ascii,
            );
        }

        drawing.overlay(&line_layer, DrawingCoord { x: 0, y: 0 }, self.options.use_ascii);
        drawing.overlay(&corner_layer, DrawingCoord { x: 0, y: 0 }, self.options.use_ascii);
        drawing.overlay(
            &arrow_head_layer,
            DrawingCoord { x: 0, y: 0 },
            self.options.use_ascii,
        );
        drawing.overlay(&box_start_layer, DrawingCoord { x: 0, y: 0 }, self.options.use_ascii);
        drawing.overlay(&label_layer, DrawingCoord { x: 0, y: 0 }, self.options.use_ascii);
    }

    fn draw_path(
        &self,
        path: &[GridCoord],
    ) -> (Drawing, Vec<Vec<DrawingCoord>>, Vec<Direction>) {
        let mut d = self.drawing.blank_like();
        let mut lines_drawn = Vec::new();
        let mut line_dirs = Vec::new();
        let mut previous = path[0];

        for next in path.iter().skip(1) {
            let prev_coord = self.grid_to_drawing_coord(previous, None);
            let next_coord = self.grid_to_drawing_coord(*next, None);
            if prev_coord == next_coord {
                previous = *next;
                continue;
            }
            let dir = determine_direction(
                GenericCoord {
                    x: previous.x,
                    y: previous.y,
                },
                GenericCoord {
                    x: next.x,
                    y: next.y,
                },
            );
            let mut segment =
                d.draw_line(prev_coord, next_coord, 1, -1, self.options.use_ascii);
            if segment.is_empty() {
                segment.push(prev_coord);
            }
            lines_drawn.push(segment.clone());
            line_dirs.push(dir);
            previous = *next;
        }

        (d, lines_drawn, line_dirs)
    }

    fn draw_box_start(
        &self,
        path: &[GridCoord],
        first_line: &[DrawingCoord],
    ) -> Drawing {
        let mut d = self.drawing.blank_like();
        if self.options.use_ascii || path.len() < 2 || first_line.is_empty() {
            return d;
        }
        let from = first_line[0];
        let dir = determine_direction(
            GenericCoord {
                x: path[0].x,
                y: path[0].y,
            },
            GenericCoord {
                x: path[1].x,
                y: path[1].y,
            },
        );
        match dir {
            Direction::Up => d.set(
                DrawingCoord {
                    x: from.x,
                    y: from.y + 1,
                },
                "┴",
            ),
            Direction::Down => d.set(
                DrawingCoord {
                    x: from.x,
                    y: from.y - 1,
                },
                "┬",
            ),
            Direction::Left => d.set(
                DrawingCoord {
                    x: from.x + 1,
                    y: from.y,
                },
                "┤",
            ),
            Direction::Right => d.set(
                DrawingCoord {
                    x: from.x - 1,
                    y: from.y,
                },
                "├",
            ),
            _ => {}
        }
        d
    }

    fn draw_arrow_head(
        &self,
        line: &[DrawingCoord],
        fallback: Direction,
    ) -> Drawing {
        let mut d = self.drawing.blank_like();
        if line.is_empty() {
            return d;
        }
        let from = line[0];
        let last_pos = line[line.len() - 1];
        let mut dir = determine_direction(
            GenericCoord {
                x: from.x,
                y: from.y,
            },
            GenericCoord {
                x: last_pos.x,
                y: last_pos.y,
            },
        );
        if line.len() == 1 || dir == Direction::Middle {
            dir = fallback;
        }

        let char = if self.options.use_ascii {
            match dir {
                Direction::Up => "^",
                Direction::Down => "v",
                Direction::Left => "<",
                Direction::Right => ">",
                _ => match fallback {
                    Direction::Up => "^",
                    Direction::Down => "v",
                    Direction::Left => "<",
                    Direction::Right => ">",
                    _ => "*",
                },
            }
        } else {
            match dir {
                Direction::Up => "▲",
                Direction::Down => "▼",
                Direction::Left => "◄",
                Direction::Right => "►",
                Direction::UpperRight => "◥",
                Direction::UpperLeft => "◤",
                Direction::LowerRight => "◢",
                Direction::LowerLeft => "◣",
                _ => match fallback {
                    Direction::Up => "▲",
                    Direction::Down => "▼",
                    Direction::Left => "◄",
                    Direction::Right => "►",
                    Direction::UpperRight => "◥",
                    Direction::UpperLeft => "◤",
                    Direction::LowerRight => "◢",
                    Direction::LowerLeft => "◣",
                    _ => "●",
                },
            }
        };

        d.set(last_pos, char);
        d
    }

    fn draw_corners(&self, path: &[GridCoord]) -> Drawing {
        let mut d = self.drawing.blank_like();
        if path.len() < 3 {
            return d;
        }
        for idx in 1..path.len() - 1 {
            let coord = path[idx];
            let drawing_coord = self.grid_to_drawing_coord(coord, None);
            let prev_dir = determine_direction(
                GenericCoord {
                    x: path[idx - 1].x,
                    y: path[idx - 1].y,
                },
                GenericCoord {
                    x: coord.x,
                    y: coord.y,
                },
            );
            let next_dir = determine_direction(
                GenericCoord {
                    x: coord.x,
                    y: coord.y,
                },
                GenericCoord {
                    x: path[idx + 1].x,
                    y: path[idx + 1].y,
                },
            );

            let corner = if self.options.use_ascii {
                "+"
            } else {
                match (prev_dir, next_dir) {
                    (Direction::Right, Direction::Down)
                    | (Direction::Up, Direction::Left) => "┐",
                    (Direction::Right, Direction::Up)
                    | (Direction::Down, Direction::Left) => "┘",
                    (Direction::Left, Direction::Down)
                    | (Direction::Up, Direction::Right) => "┌",
                    (Direction::Left, Direction::Up)
                    | (Direction::Down, Direction::Right) => "└",
                    _ => "+",
                }
            };

            d.set(drawing_coord, corner);
        }
        d
    }

    fn draw_arrow_label(&self, edge: &Edge) -> Drawing {
        let mut d = self.drawing.blank_like();
        if edge.text.is_empty() || edge.label_line.len() < 2 {
            return d;
        }
        let drawing_line = self.line_to_drawing(&edge.label_line);
        d.draw_text_on_line(&drawing_line, &edge.text);
        d
    }

    fn with_coords_overlay(&self, drawing: Drawing) -> Drawing {
        let (max_x, max_y) = drawing.size();
        let mut debug = Drawing::new(max_x + 2, max_y + 2);
        for x in 0..=max_x {
            debug.set(
                DrawingCoord { x: (x + 2) as i32, y: 0 },
                format!("{}", x % 10),
            );
        }
        for y in 0..=max_y {
            debug.set(
                DrawingCoord { x: 0, y: (y + 1) as i32 },
                format!("{}", y % 10),
            );
        }
        debug.overlay(
            &drawing,
            DrawingCoord { x: 1, y: 1 },
            self.options.use_ascii,
        );
        debug
    }

    fn get_path(
        &self,
        from: GridCoord,
        to: GridCoord,
    ) -> Result<Vec<GridCoord>> {
        let mut frontier = BinaryHeap::new();
        frontier.push(QueueItem {
            priority: 0,
            coord: from,
        });

        let mut came_from: HashMap<GridCoord, GridCoord> = HashMap::new();
        let mut cost_so_far: HashMap<GridCoord, i32> = HashMap::new();
        came_from.insert(from, from);
        cost_so_far.insert(from, 0);

        let directions = [
            GridCoord { x: 1, y: 0 },
            GridCoord { x: -1, y: 0 },
            GridCoord { x: 0, y: 1 },
            GridCoord { x: 0, y: -1 },
        ];

        while let Some(current) = frontier.pop() {
            if current.coord == to {
                let mut path = Vec::new();
                let mut curr = current.coord;
                path.push(curr);
                while curr != from {
                    curr = came_from[&curr];
                    path.push(curr);
                }
                path.reverse();
                return Ok(path);
            }

            for dir in &directions {
                let next = GridCoord {
                    x: current.coord.x + dir.x,
                    y: current.coord.y + dir.y,
                };

                if !self.is_free_in_grid(next) && next != to {
                    continue;
                }

                let new_cost = cost_so_far[&current.coord] + 1;
                if cost_so_far
                    .get(&next)
                    .map(|cost| new_cost < *cost)
                    .unwrap_or(true)
                {
                    cost_so_far.insert(next, new_cost);
                    let priority = new_cost + heuristic(next, to);
                    frontier.push(QueueItem { priority, coord: next });
                    came_from.insert(next, current.coord);
                }
            }
        }

        Err(anyhow!("no path found from {:?} to {:?}", from, to))
    }

    fn is_free_in_grid(&self, coord: GridCoord) -> bool {
        if coord.x < 0 || coord.y < 0 {
            return false;
        }
        !self.grid.contains_key(&coord)
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

fn merge_path(path: Vec<GridCoord>) -> Vec<GridCoord> {
    if path.len() <= 2 {
        return path;
    }
    let mut result = Vec::with_capacity(path.len());
    result.push(path[0]);
    let mut prev = path[0];
    let mut curr = path[1];
    for step in path.into_iter().skip(2) {
        let prev_dir = determine_direction(
            GenericCoord { x: prev.x, y: prev.y },
            GenericCoord { x: curr.x, y: curr.y },
        );
        let dir = determine_direction(
            GenericCoord { x: curr.x, y: curr.y },
            GenericCoord { x: step.x, y: step.y },
        );
        if prev_dir != dir {
            result.push(curr);
        }
        prev = curr;
        curr = step;
    }
    result.push(curr);
    result
}

fn heuristic(a: GridCoord, b: GridCoord) -> i32 {
    let abs_x = (a.x - b.x).abs();
    let abs_y = (a.y - b.y).abs();
    if abs_x == 0 || abs_y == 0 {
        abs_x + abs_y
    } else {
        abs_x + abs_y + 1
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct QueueItem {
    priority: i32,
    coord: GridCoord,
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

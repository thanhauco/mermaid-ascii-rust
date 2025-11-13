use std::collections::HashMap;

use crate::render::geom::{determine_direction, DrawingCoord, Direction, GenericCoord};

#[derive(Clone, Debug)]
pub struct Drawing {
    cells: Vec<Vec<String>>,
}

impl Drawing {
    pub fn new(width: usize, height: usize) -> Drawing {
        let mut cells = Vec::with_capacity(width + 1);
        for _ in 0..=width {
            cells.push(vec![" ".to_string(); height + 1]);
        }
        Drawing { cells }
    }

    pub fn empty() -> Drawing {
        Drawing::new(0, 0)
    }

    pub fn size(&self) -> (usize, usize) {
        let max_x = self.cells.len().saturating_sub(1);
        let max_y = if self.cells.is_empty() {
            0
        } else {
            self.cells[0].len().saturating_sub(1)
        };
        (max_x, max_y)
    }

    pub fn ensure_size(&mut self, width: usize, height: usize) {
        if self.cells.is_empty() {
            *self = Drawing::new(width, height);
            return;
        }

        if self.cells.len() <= width {
            let current_height = self.cells[0].len();
            for _ in self.cells.len()..=width {
                self.cells.push(vec![" ".to_string(); current_height]);
            }
        }

        if self.cells[0].len() <= height {
            for column in &mut self.cells {
                column.resize(height + 1, " ".to_string());
            }
        }
    }

    pub fn get(&self, coord: DrawingCoord) -> &str {
        let x = coord.x.max(0) as usize;
        let y = coord.y.max(0) as usize;
        &self.cells[x][y]
    }

    pub fn set(&mut self, coord: DrawingCoord, value: impl Into<String>) {
        let x = coord.x.max(0) as usize;
        let y = coord.y.max(0) as usize;
        self.ensure_size(x, y);
        self.cells[x][y] = value.into();
    }

    pub fn draw_text(&mut self, start: DrawingCoord, text: &str) {
        let mut x = start.x;
        let y = start.y;
        self.ensure_size((start.x + text.len() as i32) as usize, y as usize);
        for ch in text.chars() {
            self.cells[x as usize][y as usize] = ch.to_string();
            x += 1;
        }
    }

    pub fn draw_text_on_line(&mut self, line: &[DrawingCoord], label: &str) {
        if line.len() < 2 || label.is_empty() {
            return;
        }
        let first = line[0];
        let last = line[line.len() - 1];
        let (min_x, max_x) = if first.x > last.x {
            (last.x, first.x)
        } else {
            (first.x, last.x)
        };
        let (min_y, max_y) = if first.y > last.y {
            (last.y, first.y)
        } else {
            (first.y, last.y)
        };
        let middle_x = min_x + (max_x - min_x) / 2;
        let middle_y = min_y + (max_y - min_y) / 2;
        let start_x = middle_x - (label.len() as i32) / 2;
        let start = DrawingCoord {
            x: start_x,
            y: middle_y,
        };
        self.draw_text(start, label);
    }

    pub fn draw_line(
        &mut self,
        from: DrawingCoord,
        to: DrawingCoord,
        offset_from: i32,
        offset_to: i32,
        use_ascii: bool,
    ) -> Vec<DrawingCoord> {
        let mut drawn = Vec::new();
        let dir = determine_direction(
            GenericCoord {
                x: from.x,
                y: from.y,
            },
            GenericCoord { x: to.x, y: to.y },
        );
        let mut x = from.x;
        let mut y = from.y;
        let mut step = |x: i32, y: i32, value: &str, drawn: &mut Vec<DrawingCoord>| {
            let coord = DrawingCoord { x, y };
            self.set(coord, value.to_string());
            drawn.push(coord);
        };

        match dir {
            Direction::Up => {
                for py in (to.y - offset_to)..=(from.y - offset_from) {
                    step(x, py, if use_ascii { "|" } else { "│" }, &mut drawn);
                }
            }
            Direction::Down => {
                for py in (from.y + offset_from)..=(to.y + offset_to) {
                    step(x, py, if use_ascii { "|" } else { "│" }, &mut drawn);
                }
            }
            Direction::Left => {
                for px in (to.x - offset_to)..=(from.x - offset_from) {
                    step(px, y, if use_ascii { "-" } else { "─" }, &mut drawn);
                }
            }
            Direction::Right => {
                for px in (from.x + offset_from)..=(to.x + offset_to) {
                    step(px, y, if use_ascii { "-" } else { "─" }, &mut drawn);
                }
            }
            Direction::UpperLeft => {
                let mut curr_x = from.x;
                let mut curr_y = from.y - offset_from;
                while curr_x >= to.x - offset_to && curr_y >= to.y - offset_to {
                    step(
                        curr_x,
                        curr_y,
                        if use_ascii { "\\" } else { "╲" },
                        &mut drawn,
                    );
                    curr_x -= 1;
                    curr_y -= 1;
                }
            }
            Direction::UpperRight => {
                let mut curr_x = from.x;
                let mut curr_y = from.y - offset_from;
                while curr_x <= to.x + offset_to && curr_y >= to.y - offset_to {
                    step(
                        curr_x,
                        curr_y,
                        if use_ascii { "/" } else { "╱" },
                        &mut drawn,
                    );
                    curr_x += 1;
                    curr_y -= 1;
                }
            }
            Direction::LowerLeft => {
                let mut curr_x = from.x;
                let mut curr_y = from.y + offset_from;
                while curr_x >= to.x - offset_to && curr_y <= to.y + offset_to {
                    step(
                        curr_x,
                        curr_y,
                        if use_ascii { "/" } else { "╱" },
                        &mut drawn,
                    );
                    curr_x -= 1;
                    curr_y += 1;
                }
            }
            Direction::LowerRight => {
                let mut curr_x = from.x;
                let mut curr_y = from.y + offset_from;
                while curr_x <= to.x + offset_to && curr_y <= to.y + offset_to {
                    step(
                        curr_x,
                        curr_y,
                        if use_ascii { "\\" } else { "╲" },
                        &mut drawn,
                    );
                    curr_x += 1;
                    curr_y += 1;
                }
            }
            Direction::Middle => {}
        }

        if drawn.is_empty() {
            drawn.push(from);
        }
        drawn
    }

    pub fn merge_with(
        base: &Drawing,
        offset: DrawingCoord,
        drawings: &[Drawing],
        use_ascii: bool,
    ) -> Drawing {
        let mut max_x = base.cells.len().saturating_sub(1);
        let mut max_y = if base.cells.is_empty() {
            0
        } else {
            base.cells[0].len().saturating_sub(1)
        };

        for d in drawings {
            let (dx, dy) = d.size();
            max_x = max_x.max(dx + offset.x as usize);
            max_y = max_y.max(dy + offset.y as usize);
        }

        let mut merged = Drawing::new(max_x, max_y);
        merged.overlay(base, DrawingCoord { x: 0, y: 0 }, use_ascii);
        for d in drawings {
            merged.overlay(d, offset, use_ascii);
        }
        merged
    }

    pub fn overlay(&mut self, other: &Drawing, offset: DrawingCoord, use_ascii: bool) {
        let start_x = offset.x.max(0) as usize;
        let start_y = offset.y.max(0) as usize;
        let (other_max_x, other_max_y) = other.size();
        self.ensure_size(
            start_x + other_max_x,
            start_y + other_max_y,
        );

        for x in 0..=other_max_x {
            for y in 0..=other_max_y {
                let value = &other.cells[x][y];
                if value == " " {
                    continue;
                }
                let target_coord = DrawingCoord {
                    x: (start_x + x) as i32,
                    y: (start_y + y) as i32,
                };
                let current = self.get(target_coord).to_string();
                if !use_ascii && is_junction_char(value) && is_junction_char(&current) {
                    self.set(target_coord, merge_junctions(&current, value));
                } else {
                    self.set(target_coord, value.clone());
                }
            }
        }
    }

    pub fn to_string(&self) -> String {
        let (max_x, max_y) = self.size();
        let mut builder = String::new();
        for y in 0..=max_y {
            for x in 0..=max_x {
                builder.push_str(&self.cells[x][y]);
            }
            if y != max_y {
                builder.push('\n');
            }
        }
        builder
    }
}

const JUNCTION_CHARS: [&str; 15] = [
    "─", "│", "┌", "┐", "└", "┘", "├", "┤", "┬", "┴", "┼", "╴", "╵", "╶", "╷",
];

fn is_junction_char(c: &str) -> bool {
    JUNCTION_CHARS.iter().any(|jc| jc == &c)
}

fn merge_junctions(current: &str, new_char: &str) -> String {
    let mut map: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    let mut insert = |base: &str, pairs: &[(&str, &str)]| {
        let entry = map.entry(base).or_insert_with(HashMap::new);
        for (with, result) in pairs {
            entry.insert(*with, *result);
        }
    };

    insert(
        "─",
        &[
            ("│", "┼"),
            ("┌", "┬"),
            ("┐", "┬"),
            ("└", "┴"),
            ("┘", "┴"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┬", "┬"),
            ("┴", "┴"),
        ],
    );
    insert(
        "│",
        &[
            ("─", "┼"),
            ("┌", "├"),
            ("┐", "┤"),
            ("└", "├"),
            ("┘", "┤"),
            ("├", "├"),
            ("┤", "┤"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    insert(
        "┌",
        &[
            ("─", "┬"),
            ("│", "├"),
            ("┐", "┬"),
            ("└", "├"),
            ("┘", "┼"),
            ("├", "├"),
            ("┤", "┼"),
            ("┬", "┬"),
            ("┴", "┼"),
        ],
    );
    insert(
        "┐",
        &[
            ("─", "┬"),
            ("│", "┤"),
            ("┌", "┬"),
            ("└", "┼"),
            ("┘", "┤"),
            ("├", "┼"),
            ("┤", "┤"),
            ("┬", "┬"),
            ("┴", "┼"),
        ],
    );
    insert(
        "└",
        &[
            ("─", "┴"),
            ("│", "├"),
            ("┌", "├"),
            ("┐", "┼"),
            ("┘", "┴"),
            ("├", "├"),
            ("┤", "┼"),
            ("┬", "┼"),
            ("┴", "┴"),
        ],
    );
    insert(
        "┘",
        &[
            ("─", "┴"),
            ("│", "┤"),
            ("┌", "┼"),
            ("┐", "┤"),
            ("└", "┴"),
            ("├", "┼"),
            ("┤", "┤"),
            ("┬", "┼"),
            ("┴", "┴"),
        ],
    );
    insert(
        "├",
        &[
            ("─", "┼"),
            ("│", "├"),
            ("┌", "├"),
            ("┐", "┼"),
            ("└", "├"),
            ("┘", "┼"),
            ("┤", "┼"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    insert(
        "┤",
        &[
            ("─", "┼"),
            ("│", "┤"),
            ("┌", "┼"),
            ("┐", "┤"),
            ("└", "┼"),
            ("┘", "┤"),
            ("├", "┼"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    insert(
        "┬",
        &[
            ("─", "┬"),
            ("│", "┼"),
            ("┌", "┬"),
            ("┐", "┬"),
            ("└", "┼"),
            ("┘", "┼"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┴", "┼"),
        ],
    );
    insert(
        "┴",
        &[
            ("─", "┴"),
            ("│", "┼"),
            ("┌", "┼"),
            ("┐", "┼"),
            ("└", "┴"),
            ("┘", "┴"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┬", "┼"),
        ],
    );

    map.get(current)
        .and_then(|inner| inner.get(new_char))
        .map(|s| s.to_string())
        .unwrap_or_else(|| current.to_string())
}

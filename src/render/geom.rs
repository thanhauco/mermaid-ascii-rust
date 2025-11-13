use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy)]
pub struct DrawingCoord {
    pub x: i32,
    pub y: i32,
}

impl fmt::Debug for GridCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GridCoord")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl fmt::Debug for DrawingCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawingCoord")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl PartialEq for GridCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for GridCoord {}

impl Hash for GridCoord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl PartialEq for DrawingCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for DrawingCoord {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpperRight,
    UpperLeft,
    LowerRight,
    LowerLeft,
    Middle,
}

impl Direction {
    pub fn opposite(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::UpperRight => Direction::LowerLeft,
            Direction::UpperLeft => Direction::LowerRight,
            Direction::LowerRight => Direction::UpperLeft,
            Direction::LowerLeft => Direction::UpperRight,
            Direction::Middle => Direction::Middle,
        }
    }
}

impl GridCoord {
    pub fn direction(self, dir: Direction) -> GridCoord {
        match dir {
            Direction::Up => GridCoord {
                x: self.x + 1,
                y: self.y,
            },
            Direction::Down => GridCoord {
                x: self.x + 1,
                y: self.y + 2,
            },
            Direction::Left => GridCoord {
                x: self.x,
                y: self.y + 1,
            },
            Direction::Right => GridCoord {
                x: self.x + 2,
                y: self.y + 1,
            },
            Direction::UpperRight => GridCoord {
                x: self.x + 2,
                y: self.y,
            },
            Direction::UpperLeft => GridCoord {
                x: self.x,
                y: self.y,
            },
            Direction::LowerRight => GridCoord {
                x: self.x + 2,
                y: self.y + 2,
            },
            Direction::LowerLeft => GridCoord {
                x: self.x,
                y: self.y + 2,
            },
            Direction::Middle => self,
        }
    }
}

impl DrawingCoord {
    pub fn direction(self, dir: Direction) -> DrawingCoord {
        match dir {
            Direction::Up => DrawingCoord {
                x: self.x + 1,
                y: self.y,
            },
            Direction::Down => DrawingCoord {
                x: self.x + 1,
                y: self.y + 2,
            },
            Direction::Left => DrawingCoord {
                x: self.x,
                y: self.y + 1,
            },
            Direction::Right => DrawingCoord {
                x: self.x + 2,
                y: self.y + 1,
            },
            Direction::UpperRight => DrawingCoord {
                x: self.x + 2,
                y: self.y,
            },
            Direction::UpperLeft => DrawingCoord {
                x: self.x,
                y: self.y,
            },
            Direction::LowerRight => DrawingCoord {
                x: self.x + 2,
                y: self.y + 2,
            },
            Direction::LowerLeft => DrawingCoord {
                x: self.x,
                y: self.y + 2,
            },
            Direction::Middle => self,
        }
    }
}

pub fn determine_direction(from: GenericCoord, to: GenericCoord) -> Direction {
    if from.x == to.x {
        if from.y < to.y {
            Direction::Down
        } else {
            Direction::Up
        }
    } else if from.y == to.y {
        if from.x < to.x {
            Direction::Right
        } else {
            Direction::Left
        }
    } else if from.x < to.x {
        if from.y < to.y {
            Direction::LowerRight
        } else {
            Direction::UpperRight
        }
    } else if from.y < to.y {
        Direction::LowerLeft
    } else {
        Direction::UpperLeft
    }
}

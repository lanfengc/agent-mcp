use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Bounds {
    /// Parses the `[left,top][right,bottom]` format used by uiautomator.
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value.strip_prefix('[')?;
        let (left, rest) = rest.split_once(',')?;
        let (top, rest) = rest.split_once("][")?;
        let (right, rest) = rest.split_once(',')?;
        let bottom = rest.strip_suffix(']')?;
        let bounds = Self {
            left: left.parse().ok()?,
            top: top.parse().ok()?,
            right: right.parse().ok()?,
            bottom: bottom.parse().ok()?,
        };
        if bounds.right < bounds.left || bounds.bottom < bounds.top {
            return None;
        }
        Some(bounds)
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn center(&self) -> Point {
        Point {
            x: self.left + self.width() / 2,
            y: self.top + self.height() / 2,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width() <= 0 || self.height() <= 0
    }
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod tests;

//! Small, terminal-independent collision primitives.

/// An axis-aligned rectangle in terminal-cell coordinates.
///
/// Rectangles use half-open bounds: the right and bottom edges are excluded.
/// Consequently, two rectangles that only touch at an edge do not overlap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub const fn left(self) -> i32 {
        self.x
    }

    #[must_use]
    pub const fn top(self) -> i32 {
        self.y
    }

    #[must_use]
    pub const fn right(self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    /// Returns true only when the rectangles share at least one cell of area.
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.width != 0
            && self.height != 0
            && other.width != 0
            && other.height != 0
            && self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    /// Returns true when the horizontal projections share at least one cell.
    #[must_use]
    pub const fn overlaps_horizontally(self, other: Self) -> bool {
        self.width != 0
            && other.width != 0
            && self.left() < other.right()
            && self.right() > other.left()
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;

    #[test]
    fn rectangles_overlap_with_positive_area() {
        let first = Rect::new(4, 5, 5, 3);
        let second = Rect::new(8, 7, 5, 3);

        assert!(first.overlaps(second));
        assert!(second.overlaps(first));
    }

    #[test]
    fn contained_rectangle_overlaps() {
        let outer = Rect::new(0, 0, 10, 10);
        let inner = Rect::new(2, 3, 2, 4);

        assert!(outer.overlaps(inner));
        assert!(inner.overlaps(outer));
    }

    #[test]
    fn horizontal_edge_contact_is_not_overlap() {
        let left = Rect::new(0, 0, 5, 3);
        let right = Rect::new(5, 0, 5, 3);

        assert!(!left.overlaps(right));
        assert!(!right.overlaps(left));
    }

    #[test]
    fn vertical_edge_contact_is_not_overlap() {
        let top = Rect::new(0, 0, 5, 3);
        let bottom = Rect::new(0, 3, 5, 3);

        assert!(!top.overlaps(bottom));
        assert!(!bottom.overlaps(top));
    }

    #[test]
    fn separated_rectangles_do_not_overlap() {
        let first = Rect::new(1, 2, 5, 3);
        let second = Rect::new(20, 15, 5, 3);

        assert!(!first.overlaps(second));
    }

    #[test]
    fn negative_offscreen_coordinates_work() {
        let partly_visible = Rect::new(-2, -1, 5, 3);
        let visible = Rect::new(0, 0, 5, 3);
        let fully_offscreen = Rect::new(-8, -8, 2, 2);

        assert!(partly_visible.overlaps(visible));
        assert!(!fully_offscreen.overlaps(visible));
    }

    #[test]
    fn zero_area_rectangle_never_overlaps() {
        let empty = Rect::new(2, 2, 0, 3);
        let solid = Rect::new(0, 0, 5, 5);

        assert!(!empty.overlaps(solid));
        assert!(!solid.overlaps(empty));
    }

    #[test]
    fn horizontal_projection_is_independent_of_vertical_position() {
        let upper = Rect::new(10, -100, 5, 3);
        let lower = Rect::new(12, 100, 5, 3);
        let separate = Rect::new(15, 100, 5, 3);

        assert!(upper.overlaps_horizontally(lower));
        assert!(!upper.overlaps_horizontally(separate));
    }
}
